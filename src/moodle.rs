use std::{
  collections::HashMap,
  fmt,
  fs::File,
  io,
  path::{Path, PathBuf},
  result,
};

use chrono::{serde::ts_seconds, DateTime, Utc};
use reqwest::blocking::Client;
use serde::{de, Deserialize, Deserializer, Serialize};
use tabled::{settings::Style, Table, Tabled};
use url::Url;

pub struct Moodle {
  url: Url,
  token: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Moodle Error :: {0}")]
pub enum Error {
  Reqwest(#[from] reqwest::Error),
  Parse(#[from] url::ParseError),
  JsonDeserialize(#[from] serde_json::Error),
  IO(#[from] io::Error),

  #[error("Login Error :: {0}")]
  Login(String),

  #[error("{0:?} (id: {1}) not found!")]
  NotFound(NotFound, u64),
}

#[derive(Debug)]
pub enum NotFound {
  Assignment,
  Course,
  User,
}

pub type Result<T> = result::Result<T, Error>;

const LOGIN_PATH: &str = "login/token.php?service=moodle_mobile_app";
const WS_PATH: &str = "webservice/rest/server.php?moodlewsrestformat=json";

#[derive(Deserialize)]
struct LoginResponse {
  token: Option<String>,

  #[serde(default)]
  error: String,
}

impl Moodle {
  pub fn new(base_url: &Url, username: &str, password: &str) -> Result<Moodle> {
    let login = serde_json::from_reader::<_, LoginResponse>(
      Client::new()
        .post(base_url.join(LOGIN_PATH)?)
        .form(&HashMap::from([
          ("username", username),
          ("password", password),
        ]))
        .send()?,
    )?;

    if let Some(token) = login.token {
      Ok(Moodle {
        url: base_url.join(WS_PATH)?,
        token,
      })
    } else {
      Err(Error::Login(login.error))
    }
  }

  fn post<T>(&self, params: HashMap<&str, &str>) -> Result<T>
  where
    T: de::DeserializeOwned,
  {
    let mut params = params;
    params.insert("wstoken", &self.token);

    Ok(serde_json::from_reader::<_, T>(
      Client::new().post(self.url.clone()).form(&params).send()?,
    )?)
  }

  pub fn upload_grade(
    &self,
    assignment: &MAssignment,
    user: &MUser,
    grade: f32,
    feedback: Option<&str>,
  ) -> Result<()> {
    let assignment_id = assignment.id.to_string();
    let user_id = user.id.to_string();
    let grade = grade.max(0.0).min(assignment.max_grade).to_string();
    let params = HashMap::from([
      ("wsfunction", "mod_assign_save_grade"),
      ("assignmentid", &assignment_id),
      ("userid", &user_id),
      ("grade", &grade),
      ("attemptnumber", "-1"),
      ("addattempt", "0"),
      ("workflowstate", ""),
      ("applytoall", "0"),
      (
        "plugindata[assignfeedbackcomments_editor][text]",
        feedback.unwrap_or_default().trim(),
      ),
      ("plugindata[assignfeedbackcomments_editor][format]", "2"),
    ]);

    self.post(params)
  }

  pub fn download_file(
    &self,
    file: &SubmissionFile,
    path: impl AsRef<Path>,
  ) -> Result<()> {
    Client::new()
      .post(file.fileurl.as_str())
      .form(&HashMap::from([("token", &self.token)]))
      .send()?
      .copy_to(&mut File::create(path)?)?;

    Ok(())
  }
}

#[derive(Deserialize)]
struct AssignmentsResponse {
  courses: Vec<MCourse>,
}

#[derive(Deserialize)]
pub struct MCourse {
  id: u64,
  pub fullname: String,
  assignments: Vec<MAssignment>,
}

impl fmt::Display for MCourse {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "Course (id: {}) :: {}", self.id, self.fullname)?;
    if self.assignments.is_empty() {
      writeln!(f, "No assignments found!")
    } else {
      writeln!(
        f,
        "{}",
        Table::new(&self.assignments).with(Style::rounded())
      )
    }
  }
}

#[derive(Deserialize, Tabled)]
pub struct MAssignment {
  #[tabled(rename = "ID")]
  pub id: u64,

  #[tabled(rename = "Name")]
  pub name: String,

  #[serde(rename = "grade")]
  #[tabled(rename = "Max Grade")]
  pub max_grade: f32,

  #[serde(with = "ts_seconds")]
  #[tabled(display_with = "display_dt", rename = "Due Date & Time")]
  pub duedate: DateTime<Utc>,
}

fn display_dt(date: &DateTime<Utc>) -> String {
  date.to_rfc3339()
}

impl MAssignment {
  #[allow(clippy::cast_sign_loss)]
  pub fn calculate_late(&self, submission: &MSubmission) -> u64 {
    submission
      .timemodified
      .signed_duration_since(self.duedate)
      .num_seconds()
      .max(0) as u64
  }
}

impl MCourse {
  pub fn get_assignment(&self, assignment_id: u64) -> Result<&MAssignment> {
    self
      .assignments
      .iter()
      .find(|a| a.id == assignment_id)
      .ok_or(Error::NotFound(NotFound::Assignment, assignment_id))
  }
}

impl Moodle {
  pub fn get_course_assignments(&self, course_id: u64) -> Result<MCourse> {
    let course_id_str = course_id.to_string();
    let params = HashMap::from([
      ("wsfunction", "mod_assign_get_assignments"),
      ("courseids[]", &course_id_str),
    ]);

    for course in self.post::<AssignmentsResponse>(params)?.courses {
      if course.id == course_id {
        return Ok(course);
      }
    }

    Err(Error::NotFound(NotFound::Course, course_id))
  }
}

#[derive(Deserialize)]
struct SubmissionsResponse {
  assignments: Vec<AssignmentSubmission>,
}

#[derive(Deserialize)]
struct AssignmentSubmission {
  assignmentid: u64,
  submissions: Vec<MSubmission>,
}

#[derive(Deserialize)]
pub struct MSubmission {
  pub userid: u64,

  #[serde(with = "ts_seconds")]
  timemodified: DateTime<Utc>,

  #[serde(deserialize_with = "deserialize_files", rename = "plugins")]
  pub files: Vec<SubmissionFile>,
}

fn deserialize_files<'de, D>(
  deserializer: D,
) -> result::Result<Vec<SubmissionFile>, D::Error>
where
  D: Deserializer<'de>,
{
  for plugin in Vec::<SubmissionPlugin>::deserialize(deserializer)? {
    if plugin.plugin_type == "file" {
      for filearea in plugin.fileareas.unwrap_or_default() {
        if filearea.area == "submission_files" {
          return Ok(filearea.files.unwrap_or_default());
        }
      }
    }
  }

  Ok(Vec::new())
}

#[derive(Deserialize)]
struct SubmissionPlugin {
  #[serde(rename = "type")]
  plugin_type: String,
  fileareas: Option<Vec<SubmissionFileArea>>,
}

#[derive(Deserialize)]
struct SubmissionFileArea {
  area: String,
  files: Option<Vec<SubmissionFile>>,
}

#[derive(Deserialize)]
pub struct SubmissionFile {
  filename: String,
  fileurl: Url,

  #[serde(deserialize_with = "deserialize_filepath")]
  filepath: PathBuf,
}

fn deserialize_filepath<'de, D>(
  deserializer: D,
) -> result::Result<PathBuf, D::Error>
where
  D: Deserializer<'de>,
{
  Ok(
    PathBuf::deserialize(deserializer)?
      .strip_prefix("/")
      .unwrap_or(Path::new(""))
      .to_path_buf(),
  )
}

impl SubmissionFile {
  pub fn fullpath(&self) -> PathBuf {
    self.filepath.join(&self.filename)
  }
}

impl Moodle {
  pub fn get_submissions(
    &self,
    assignment_id: u64,
  ) -> Result<Vec<MSubmission>> {
    let assignment_id_str = assignment_id.to_string();
    let params = HashMap::from([
      ("wsfunction", "mod_assign_get_submissions"),
      ("assignmentids[]", &assignment_id_str),
    ]);

    for assignment_submission in
      self.post::<SubmissionsResponse>(params)?.assignments
    {
      if assignment_submission.assignmentid == assignment_id {
        return Ok(assignment_submission.submissions);
      }
    }

    Err(Error::NotFound(NotFound::Assignment, assignment_id))
  }
}

#[derive(Deserialize, Serialize)]
struct UserResponse(Vec<MUser>);

#[derive(Deserialize, Serialize)]
pub struct MUser {
  id: u64,
  fullname: String,
  email: String,
}

impl Moodle {
  pub fn get_user(&self, user_id: u64) -> Result<MUser> {
    let user_id_str = user_id.to_string();
    let params = HashMap::from([
      ("wsfunction", "core_user_get_users_by_field"),
      ("field", "id"),
      ("values[]", &user_id_str),
    ]);

    for user in self.post::<UserResponse>(params)?.0 {
      if user.id == user_id {
        return Ok(user);
      }
    }

    Err(Error::NotFound(NotFound::User, user_id))
  }
}
