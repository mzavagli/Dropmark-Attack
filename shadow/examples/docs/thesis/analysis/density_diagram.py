import re

# Sample log data as a list of strings (each string is a line from the log)
with open('logs/output_all.txt') as infile:
    logs = infile.readlines()

# Function to parse the timestamp from a log line
def parse_time(log_line):
    # Extract the timestamp following the "at time" phrase using regex
    match = re.search(r"at time (\d+)", log_line)
    if match:
        return int(match.group(1))
    return None

last_sending_time = None
time_deltas = []
relay_early_passed = 0

for log_line in logs:
    if "relay_early_passed":
        relay_early_passed = 1
    elif "Sending" in log_line:
        # Update last sending time when a "Sending" log is encountered
        last_sending_time = parse_time(log_line)
    elif "command 9" in log_line:
        # Compute delta when a "command 9" log is encountered
        command_time = parse_time(log_line)
        if last_sending_time and command_time:
            # Calculate the time difference and store it
            delta = command_time - last_sending_time
            time_deltas.append(delta)

# Print all computed time deltas
print("All time deltas:", time_deltas)
