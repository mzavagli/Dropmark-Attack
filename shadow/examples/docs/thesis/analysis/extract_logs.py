EXTRACT_EXIT = True
EXTRACT_RELAY = True

number_of_relay = 12
number_of_exit = 4

OUTPUT_PATH = "logs/output_all.txt"

content = ""

if (EXTRACT_EXIT):
    for i in range(number_of_exit):

        EXIT_LOG_PATH = f"../shadow.data/hosts/exit{i+1}/tor.1000.stdout"

        content += f"--- EXIT{i+1} ---\n"

        with open(EXIT_LOG_PATH) as infile:
            logs = infile.readlines()

        for line in logs:
            if "DROPMARK" in line:
                content += line

if (EXTRACT_RELAY):
    for i in range(number_of_relay):

        RELAY_LOG_PATH = f"../shadow.data/hosts/relay{i+1}/tor.1000.stdout"

        content += f"--- RELAY{i+1} ---\n"

        with open(RELAY_LOG_PATH) as infile:
            logs = infile.readlines()

        for line in logs:
            if "DROPMARK" in line:
                content += line

with open(OUTPUT_PATH, "w") as outfile:
    outfile.write(content)