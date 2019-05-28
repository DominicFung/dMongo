@ECHO OFF

ECHO "STARTING MONGO .."

IF [%1] == [] GOTO ERROR
if [%2] == [] GOTO ERROR

start mongod --dbpath %1 --port %2
ECHO "Done."
EXIT 0

:ERROR
ECHO "ERROR:"
ECHO "[param1] = mongo database data path --dbpath"
ECHO "[param2] = mongo port --port"
EXIT 1
