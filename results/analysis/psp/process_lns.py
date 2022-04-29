#
import pandas as pd

################################################################################
# Colonnes des XP
################################################################################
# Instance | Method | Status | Value | RAM | Best (s) | Proved (s) | Solution
################################################################################
# Colonnes des solutions
################################################################################
# 'id', 'value'
################################################################################

def dataset_results(data_file, solution_file):
  xp = pd.read_csv(data_file, header=0, sep='\s+\|\s+', engine='python')
  sol= pd.read_csv(solution_file, header=0, sep='\s+', engine='python')
  initial = len(xp)
  closed  = len(xp[xp["Status"].str.startswith("closed")])
  ################################################################################
  # On va etendre les infos relatives aux xp
  ################################################################################
  opt_found = []
  gap       = []
  for _,row in xp.iterrows():
    inst = row['Instance']
    rval = row['Value']
    sval = sol[sol['id'] == inst]['value'].values[0]
    gap.append((rval-sval)/sval * 100.0)
    opt_found.append((sval == rval))
  xp['OptimumFound'] = opt_found
  xp['Gap'] = gap
  ################################################################################
  # On va forcer le respect des contraintes de temps et de RAM utilisée
  ################################################################################
  xp = xp[xp['RAM'] <= 2.0]
  ################################################################################
  # Filtrage des non valeurs
  ################################################################################
  bst = xp[xp['Best (s)'] != "N.A."].astype({'Best (s)': 'float64'})
  return bst

def process_optimum(dataset, output_file):
  data = dataset[dataset["OptimumFound"]==True].sort_values(by=['Best (s)'])
  with open(output_file, 'w') as f:
    for i,(_,row) in enumerate(data.iterrows()):
      print("{};{}".format(row["Best (s)"], i), file=f)

def process_gap(dataset, output_file, tolerance=0):
  data = dataset[dataset["Gap"].abs()<=tolerance].sort_values(by=['Best (s)'])
  with open(output_file, 'w') as f:
    for i,(_,row) in enumerate(data.iterrows()):
      print("{};{}".format(row["Best (s)"], i), file=f)

def process_lns(results, output_name):
  dataset = dataset_results(results, 'best-known.csv')
  optimum = process_optimum (dataset, output_name + ".best.data")
  gap     = process_gap     (dataset, output_name + ".gap.data",  tolerance=1)

################################################################################
################################################################################
################################################################################
if __name__ == '__main__':
  process_lns("results/ddlns/lns.w10-t600.txt",      "processed/lns-w10")
  process_lns("results/ddlns/lns.w100-t600.txt",     "processed/lns-w100")
  process_lns("results/ddlns/lns.w1000-t600.txt",    "processed/lns-w1000")
  process_lns("results/ddlns/lns.w5000-t600.txt",    "processed/lns-w5000")
  process_lns("results/ddlns/lns.w10000-t600.txt",   "processed/lns-w10000")
  process_lns("results/ddlns/lns.w15000-t600.txt",   "processed/lns-w15000")
  process_lns("results/ddlns/lns.w20000-t600.txt",   "processed/lns-w20000")
  process_lns("results/ddlns/lns.w100000-t600.txt",  "processed/lns-w100000")
  process_lns("results/ddlns/lns.w1000000-t600.txt", "processed/lns-w1000000")
