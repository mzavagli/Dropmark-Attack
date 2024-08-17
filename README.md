# Dropmark Attack
This repository contains all the code requires to reproduce the results of our paper: 

**In-Lab Deanonymization Of Tor Clients - The Dropmark Attack**

## Table of Contents
- [Fetch Repository](#fetch-repository)
- [Install Custom Tor Version](#install-custom-tor-version)
- [Shadow](#shadow)
- [Dropmark Attack: Phase 1](#dropmark-attack-phase-1)
- [Dropmark Attack: Phase 2](#dropmark-attack-phase-2)

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
Return to the *thesis* directory:
```bash
cd ../..
```
Run the simulation:
```bash
./run.sh
```
Once the simulation ended successfully, go to the *analysis* directory:
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
Return to the *thesis* directory:
```bash
cd ../..
```
Run the simulation:
```bash
./run.sh
```
Once the simulation ended successfully, go to the *analysis* directory:
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
