# man-with

search and execute command and watching man page.

*Note*
`man-with` depends on `col` command because remove special character from man command output.

## Status

WORK IN PROGRESS.

## Usage

```sh
$ man-with [OPTION] <COMMAND>
```

### Options

#### -s/--size <number>

Default: 10
Set number of lines of man page viewer.

#### -p/--use_help

Default: false
Using the --help option instead of man command


## Available Keys

| Key   | Notes |
| ------| ---- |
| C-n   | Search next |
| C-p   | Search previous |
| C-c   | Exit from `man-with` and cancel execute command |
| Enter | Append command argument  |
|       | Quit and Execute command |
| Up    | Scroll up a man page |
| Down  | Scroll down a man page |
| F1    | Toggle show line number. |

### Supported Platforms

- Linux
- MacOX
