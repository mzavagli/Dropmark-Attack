from statistics import mean, median

LOGS_PATH = "logs/output_all.txt"

with open(LOGS_PATH) as log_file:
    logs = log_file.readlines()

mean_before = []
mean_after = []

for i in range(len(logs)):
    if "Spotted" in logs[i]:
        circid = logs[i-1].split(" ")[-4]
        tmp_list = []
        j = i
        while len(tmp_list) != 5:
            if logs[j].split(" ")[-4] == circid:
                tmp_list.append(int(logs[j].split(" ")[-1])/1000)
            j -= 1
        # print(tmp_list)
        mean_after.append(tmp_list[0]-tmp_list[1])
        mean_before.append(tmp_list[3]-tmp_list[4])

print(f"BEFORE DROPMARK: Mean:{mean(mean_before)}, median:{median(mean_before)}, min:{min(mean_before)}, max:{max(mean_before)}")
print(f"AFTER DROPMARK: Mean:{mean(mean_after)}, median:{median(mean_after)}, min:{min(mean_after)}, max:{max(mean_after)}")