# Moody

[![Commitizen friendly](https://img.shields.io/badge/commitizen-friendly-brightgreen.svg)](http://commitizen.github.io/cz-cli/)

Moody is a Moodle cli built to interact with assignments. Features:

- [X] List Assignments
- [X] Download Submissions
- [X] Upload Grades & Feedback

```sh
Usage: moody --base-url <BASE_URL> --username <USERNAME> --password <PASSWORD> <COMMAND>

Commands:
  list-assignments
  download-submissions
  upload-grades
  help                  Print this message or the help of the given subcommand(s)

Options:
  -b, --base-url <BASE_URL>  [env: MOODLE_BASE_URL=]
  -u, --username <USERNAME>  [env: MOODLE_USERNAME=]
  -p, --password <PASSWORD>  [env: MOODLE_PASSWORD=]
  -h, --help                 Print help
```

## Modules

Moody works with modules, that grade the submissions and provide feedback. Modules available:

- [Hasky](https://github.com/arghyadipchak/hasky) - Haskell Grader
