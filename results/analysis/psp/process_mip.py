import pandas as pd

def dataset_results(data_file):
    # INSTANCE | STATUS | UB | LB | DURATION | PEAK-MEMORY
    xp = pd.read_csv(data_file, header=0, sep='\s+\|\s+', engine='python')
    xp = xp[xp['INSTANCE'].str.startswith("10items/")]
    xp['INSTANCE'] = xp['INSTANCE'].str[8:]
    #
    sol= pd.read_csv("best-known.csv", header=0, sep='\s+', engine='python')
    #
    opt_found = []
    gap       = []
    for _,row in xp.iterrows():
      inst = int(row['INSTANCE'])
      rval = row['UB']
      sval = sol[sol['id'] == inst]['value'].values[0]
      opt_found.append((sval == rval))
      gap.append((rval-sval)/sval * 100.0)
    xp['OptimumFound'] = opt_found
    xp['Gap'] = gap
    return xp

def process_optimal(dataset, output_file):
    bst     = dataset[dataset['OptimumFound']==True]
    by_best = bst.sort_values(by=['TIME-TO-BEST'])

    with open(output_file, 'w') as f:
        for i,(_,row) in enumerate(by_best.iterrows()):
          print("{};{}".format(row["TIME-TO-BEST"], i), file=f)

def process_gap(dataset, output_file, tolerance=0):
    bst     = dataset[dataset["Gap"].abs()<=tolerance]
    by_best = bst.sort_values(by=['TIME-TO-BEST'])

    with open(output_file, 'w') as f:
        for i,(_,row) in enumerate(by_best.iterrows()):
          print("{};{}".format(row["TIME-TO-BEST"], i), file=f)

def process(datafile, output_name):
  dataset = dataset_results(datafile)
  process_optimal(dataset, output_name + ".best.data")
  process_gap    (dataset, output_name + ".gap.data", tolerance=1)
#
if __name__ == "__main__":
    process("results/mip/mip1.txt", "processed/mip1")
    process("results/mip/mip2.txt", "processed/mip2")
    process("results/mip/mip3.txt", "processed/mip3")

