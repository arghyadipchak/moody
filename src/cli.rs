use std::path::PathBuf;

use clap::{Parser, Subcommand};
use url::Url;

#[derive(Parser)]
pub struct Cli {
  #[arg(short, long, env = "MOODLE_BASE_URL")]
  pub base_url: Url,

  #[arg(short, long, env = "MOODLE_USERNAME")]
  pub username: String,

  #[arg(short, long, env = "MOODLE_PASSWORD")]
  pub password: String,

  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  ListAssignments {
    #[arg(short, long, help = "Course id")]
    course_id: u64,
  },

  DownloadSubmissions {
    #[arg(short, long, help = "Course id")]
    course_id: u64,

    #[arg(short, long, help = "Assignment id")]
    assignment_id: u64,

    #[arg(
      short,
      long,
      help = "Output file [default: assignment_{assignment_id}.yml]"
    )]
    output_file: Option<PathBuf>,
  },

  UploadGrades {
    #[arg(short, long, help = "Assignment file")]
    file: PathBuf,
  },
}
