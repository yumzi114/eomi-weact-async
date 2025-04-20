count=0
while true; do
  echo -ne "Hello $count\r" > /dev/ttyUSB0
  ((count++))
  sleep 0.07
done