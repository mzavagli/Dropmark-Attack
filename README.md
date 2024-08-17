# Dropmark Attack
This repository contains all the code requires to reproduce the results of our paper: 

**In-Lab Deanonymization Of Tor Clients - The Dropmark Attack**

## Disclaimer

**This project is intended solely for educational and research purposes**

The code and techniques provided in this repository are designed to demonstrate and analyze the dropmark attack. **Under no circumstances should this code be used for malicious purposes or to target real-world systems or networks, mainly the live Tor network**.

**By using this code, you agree to take full responsibility for your actions**. We, as the author of this repository, disclaim any liability for any misuse or damage that may arise from the use of this code. Users are strongly advised to test and analyze the code in a controlled, isolated environment, such as Shadow, and fully comply with all applicable laws and ethical guidelines.

If you are unsure of the ethical or legal implications of using this code, please refrain from using it and seek appropriate guidance.

## Table of Contents
- [Fetch Repository](#fetch-repository)
- [Install Custom Tor Version](#install-custom-tor-version)
- [Shadow](#shadow)
- [Dropmark Attack: Phase 1](#dropmark-attack-phase-1)
- [Dropmark Attack: Phase 2](#dropmark-attack-phase-2)
- [Support Scripts](#support-scripts)

## Fetch Repository
Clone this repository on your local machine:
```bash
git clone https://github.com/mzavagli/Dropmark-Attack.git
```
```bash
cd Dropmark-Attack
```

## Install Custom Tor Version
Starting from the root directory, we install our custom version of Tor, based on [the official Tor repository](https://gitlab.torproject.org/tpo/core/tor/-/tree/main):
```bash
cd tor/

./autogen.sh

./configure

(sudo) make

(sudo) make install
```
## Shadow
Make sure you have Shadow installed on your machine, [follow the official tutorial](https://shadow.github.io/docs/guide/shadow.html).

## Dropmark Attack: Phase 1
Configure Shadow to execute phase 1 on our largest topology, from the root directory:
```bash
cd shadow/examples/docs/thesis/conf/
```
Open *tor.common.torrc*, and put the configuration options:
```
SignalMethod 0
```
Open *tor.exit.torrc*, and make sure that the destination server IP address is correct:
```
WatchAddress x.x.x.x
```
Return to the *thesis/* directory:
```bash
cd ../..
```
Run the simulation:
```bash
./run.sh
```
Once the simulation ended successfully, go to the *analysis/* directory:
```bash
cd analysis
```
Extract the logs:
```bash
python3 extract_logs.py
```
Verify that 30 dropmark signals sending were triggered: 
```bash
grep 'Sending' logs/output_all.txt | wc -l
```
Verify that 30 dropmark waternark were spotted:
```bash
grep 'Spotted' logs/output_all.txt | wc -l
```
Go to the data repository:
```bash
cd ../shadow.data/hosts/torclient/
```
Verify that 30 HTTP transfer were successful for both upload and download:
```bash
grep 'stream-success' tgen.1003.stdout | wc -l
```


## Dropmark Attack: Phase 2
Configure Shadow to execute phase 2 on our largest topology, from the root directory:
```bash
cd shadow/examples/docs/thesis/conf/
```
Open *tor.common.torrc*, and put the configuration options:
```
SignalMethod 1
```
Open *tor.exit.torrc*, and make sure that the destination server IP address is correct:
```
WatchAddress x.x.x.x
```
Return to the *thesis/* directory:
```bash
cd ../..
```
Run the simulation:
```bash
./run.sh
```
Once the simulation ended successfully, go to the *analysis/* directory:
```bash
cd analysis
```
Verify that 30 dropmark watermark and 30 confirmation signal were spotted, and the corresponding uids:
```bash
./correlation.sh
```
Go to the data repository:
```bash
cd ../shadow.data/hosts/torclient/
```
See that 30 HTTP transfer were unsuccessful for both upload and download:
```bash
grep 'stream-error' tgen.1003.stdout | wc -l
```

## Support Scripts
Usage of our support scripts (in *analysis/* directory):
- Extract logs, after the simulation is completed:
```bash
python3 extract_logs.py
```
- Minimal time elapsed:
```bash
python3 compute_mean_time.py
```
- Reproduction of the plots (figure 4.1):
```bash
python3 guard_node_cells.py
```
- Compute minimum ID length required:
```bash
python3 compute_minimum_ID_length.py
```