: 

# LOAD FICO in the current environment
source /home/xgillard/fico/xpressmp/bin/xpvars.sh 

header="INSTANCE                       | STATUS     |      UB |      LB |    TIME-TO-BEST |  DURATION | PEAK-MEMORY "
function run_model() {
    model=$1 
    n=$2
    limit=$3
    echo ${header} > results/mip${model}.txt 

	#probset="05items/"
    #command="nohup mosel PIG-A-${model}.mos TIMELIMIT=\"${limit}\" FOLDER=\"${probset}\" instance=\"{}\" >> results/mip${model}.txt"
    #nohup parallel ${command} ::: $(seq -f "%03g" ${n})

	probset="10items/"
    command="nohup mosel PIG-A-${model}.mos TIMELIMIT=\"${limit}\" FOLDER=\"${probset}\" instance=\"{}\" >> results/mip${model}.txt"
    nohup parallel ${command} ::: $(seq -f "%03g" ${n})
}

n=500
limit=600
mkdir -p results
#run_model 1 ${n} ${limit}
run_model 2 ${n} ${limit}
#run_model 3 ${n} ${limit}
