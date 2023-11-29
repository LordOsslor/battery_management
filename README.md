# Linux charge control daemon

A simple daemon to change charge control thresholds for Linux laptops

## Usage:
1. Run the battery_management binary as root
2. Write to the configured pipe and...
    1. change the start threshold to `X`% with `start=X`, eg: ```echo start=20 > /tmp/battery_pipe```
    2. change the end threshold to `Y`% with `end=Y`, eg: ```echo end=80 > /tmp/battery_pipe```
3. ...?
4. Profit!
