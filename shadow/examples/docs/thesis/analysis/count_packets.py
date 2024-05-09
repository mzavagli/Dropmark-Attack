import pprint

ATTACK_ENABLED = "logs/output_injection_enabled.txt"
ATTACK_DISABLED = "logs/output_injection_disabled.txt"

count_dict = {}

with open(ATTACK_ENABLED) as infile:
    lines = infile.readlines()
    for line in lines:
        if '-' not in line:
            line = line.split(' ')
            if line[2] in count_dict.keys():
                count_dict[line[2]][0] += 1
            else:
                count_dict[line[2]] = [1, 0]


with open(ATTACK_DISABLED) as infile:
    lines = infile.readlines()
    for line in lines:
        if '-' not in line:
            line = line.split(' ')
            if line[2] in count_dict.keys():
                count_dict[line[2]][1] += 1
            else:
                count_dict[line[2]] = [0, 1]

# pprint.pp(count_dict)

res_list = []

for key in count_dict.keys():
    if count_dict[key][0] != count_dict[key][1]:
        res_list.append(f'{key} - {count_dict[key]}')

pprint.pp(res_list)