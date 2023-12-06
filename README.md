# Linux charge control daemon

A simple daemon to change charge control thresholds for Linux laptops

## Usage:
1. Run the daemon as root (for example via systemd service)
2. Write to the pipe:
    
    echo [SYNTAX] > /tmp/battery_pipe.


    Syntax examples where `X` is the start threshold and `Y` the end
    - `X to Y`
    - `X .. Y`
    - `start=X`
    - `end=Y`
    - `X..`
    - `..Y`
    - `X to`
    - `to Y`
    - `X`     (if X<50)
    - `Y`     (if 50< Y <= 100)
    - `XtoY`

## Example:
```console
# battery_management --pipe-uid 0 --pipe-gid 0 --pipe-permissions 422 --default-start 20 --default-end 80

$ echo 10 to 90 > /tmp/battery_pipe
```