
#for i in "10 100 200 300 400 500 600 700 800 900 1000 2000 3000 4000 5000 6000 7000 8000 9000 10000

prefix="compare"

probabilities="0.8 0.01"
network_sizes="10 50 100 250 500 750 1000"

#probabilities="0.8 0.5 0.1 0.01 0.001 0.0001"
#network_sizes="10 50 100 250 500 750 1000 2000 3000 4000 5000 6000 7000 8000 9000 10000 12000 1400"

for p in $probabilities
do 
  dir="compare_"$p"/"
  mkdir -p $dir 
  echo "overlay,nodes,committees_or_depth,description" >  $dir$prefix"_"$p".csv"
  for i in $network_sizes
  do
  python3 build_tests.py --num-nodes $i --failure-threshold $p --debug >> $dir$prefix"_"$p".csv"
  echo "num-nodes = $i, failure-threshold = $p"
  done
done



for p in $probabilities
do
  dir="compare_"$p"/"
  cd $dir
  mkdir configs output scripts
  cp  ../*.py  scripts
  cd scripts
  ln -s ../../config_builder/
  python3 build_cases.py  ../$prefix"_"$p".csv"
  cd ..
  python3 scripts/run_configs.py configs/
  cd ..
  echo "config gen ($p) done.."
done



for p in $probabilities
do
   dir="compare_"$p"/"
   cd $dir
   # renaming moved to `build_cases.py`
   if ls output/*.json 1> /dev/null 2>&1; then
      echo "json extension in output dir; need to rename"
      ls -l output/*.json |  awk '{print $9}' > json_files
      sed -e s/\.json//g -i json_files
      for i in `cat json_files`
      do
          mv $i.json $i.csv
      done
   fi
   rm -rf  scripts
   cd ..
   echo "renaming/cleanup ($p) done.."
done
