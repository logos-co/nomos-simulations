
#for i in "10 100 200 300 400 500 600 700 800 900 1000 2000 3000 4000 5000 6000 7000 8000 9000 10000"
prefix="compare"
for p in 0.8 0.5 0.1 0.01 0.001 0.0001
do 
  dir="compare_"$p"/"
  mkdir -p $dir 
  echo "overlay,nodes,committees_or_depth,description" >  $dir$prefix"_"$p".csv"
  for i in 10 50 100 250 500 750 1000 2000 3000 4000 5000 6000 7000 8000 9000 10000 12000 1400
  do
  python3 build_tests.py --num-nodes $i --failure-threshold $p >> $dir$prefix"_"$p".csv"
  echo "num-nodes = $i, failure-threshold = $p"
  done
done


for p in 0.8 0.5 0.1 0.01 0.001 0.0001
do
  dir="compare_"$p"/"
  cd $dir
  mkdir configs output scripts
  cp  ../*.py  scripts
  cd scripts
  ln -s ../../config_builder/   
  python3 build_cases.py  ../$prefix"_"$p".csv"
  python3 run_configs.py ../configs 
  cd ..
  rm -rf scripts
  cd ..
  echo "config gen ($p) done.."
done
