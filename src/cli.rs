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
    #[arg(short, long)]
    course_id: u64,
  },
  DownloadSubmissions {
    #[arg(short, long)]
    course_id: u64,

    #[arg(short, long)]
    assignment_id: u64,

    #[arg(short, long)]
    output_file: Option<PathBuf>,
  },
  UploadGrades {
    #[arg(short, long)]
    file: PathBuf,
  },
}
