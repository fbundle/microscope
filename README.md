# microscope

an extremely fast text editor


![screenshot](./screenshots/0_1_2.png)

## NOTE

- refer to [github.com/fbundle/wbt](https://github.com/fbundle/wbt) for WBT

- the current implementation of weight-balanced tree doesn't support efficient merging and cutting


## FEATURE SET

- basic text editor
    - navigate with `Left`, `Right`, `Up`, `Down`, `PgUp`, `PgDn`, `Home`, `End`
    - undo redo with `Ctrl+U`, `Ctrl+R`

- instant start-up, instant edit

- able to handle very large files, potentially even larger than system memory.

- able to recover from crash

- able to edit while still loading the file and exit without losing any progress

- vim-like command mode, search, goto line, etc.

## RELEASE MODEL

- initial release comes with no suffix, e.g. `0.1.7`

- bug fix releases come with suffix, e.g. `0.1.7a, 0.1.7b`

- the latest stable release should be one lower than latest releast with highest suffix. for example, if `0.1.7b` is the latest release, then `0.1.6z` is the stable release
 
## INTERACTION WITH FILE SYSTEM

0. use `microscope -h` for help

1. when user opens a file using `microscope inputfile`, the program will create a log file (journal file) stored at `<tmp>/microscope_log/<path>` where `<tmp>` is system default temporary folder

2. when user edit the file, every action will be written to log file.

3. when exit the program the log file is preserved to export

4. user can use command `:w outputfile` to write the current file into a new file, if `outputfile` is empty, it will overwrite the current file and exit

5. user use `microscope -r inputfile` to replay the log to make a new file. the program will write the output to stdout

## NOTES

update `go.mod` directly from github `GOPROXY=direct go mod tidy`




