@echo off

start mongod --dbpath D:/mongodata/du1 --port 27018
start mongod --dbpath D:/mongodata/du2 --port 27019
start mongod --dbpath D:/mongodata/du3 --port 27020

start cargo run -- -dbp 27018 -p 8081
start cargo run -- -dbp 27019 -p 8082
start cargo run -- -dbp 27020 -p 8083