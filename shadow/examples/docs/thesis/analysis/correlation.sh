#!/usr/bin/bash

python3 extract_logs.py

## grep 'Sending sign' logs/output_all.txt | sort
## sum=$(grep 'Sending sign' logs/output_all.txt | sort | wc -l) 
## echo "----------------------------------------------- Total:$sum"
grep 'Spotted wat' logs/output_all.txt | sort
sum=$(grep 'Spotted wat' logs/output_all.txt | sort | wc -l) 
echo "----------------------------------------------- Total:$sum"
grep 'Spotted_conf' logs/output_all.txt | sort
sum=$(grep 'Spotted_conf' logs/output_all.txt | sort | wc -l) 
echo "----------------------------------------------- Total:$sum"