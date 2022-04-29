# Supplementary Material

This is the supplementary material accompanying our IJCAI-22 submitted paper
``Large Neighborhood Search with Decision Diagrams''. Its content are structured
as follow:

## Findings

This folder contains a summary of the findings we made during our experimental
study. These are provided as a short-hand means for the reviewer willing to 
verify our claims about the new solutions we found which improve the objective
value of the best known solutions. These short hand summary should help the
reviewer get a quick idea of the results without taking much time digging into
the actual code and detailed results. (These are provided too so as to make 
this line of research 100% reproductible).

## Source-code Folder

This folder contains the source code of all the solvers and modes that have 
been used during our experimental study.

* `solver-ddlns` contains the source code of the generic solver which has been
  implements our method. The example `psp` and `tsptw` models which have been
  used in our experimental study are to be respectively found in the 
  `examples/psp2` and `examples/tsptw` folders. This solver is written in the
  *Rust* language (www.rust-lang.org) and must therefore be compiled using the
  `cargo` build tool. In practice, the solvers used during our experimental 
  study can be built with the following commands:
  ```
  # Solver for the PSP problem. The resulting binary which will be produced 
  # will be located at target/release/examples/psp2. To run this solver or
  # know how to use it, simply start it with the `--help` flag. It should
  # provide you with an usage message.
  cargo build --release --example psp2

  # Solver for the TSPTW problem. The resulting binary which will be produced 
  # will be located at target/release/examples/tsptw. To run this solver or
  # know how to use it, simply start it with the `--help` flag. It should
  # display an helpful usage message.
  cargo build --release --example tsptw
  ``` 
  In practices, these solvers have respectively been run with the following
  commands during our experimental study.
  ```
  # PSP. During the experiments, we let the maximum width vary but used the
  # following fixed values for the other parameters.
  # 
  # $maximum_duration  = 600 (seconds)
  # $maximum_ram_in_gb = 2   
  # $seed              = 20220103
  # $proba             = 0.1
  #
  psp2 solve -f $instance_file     \
  	--width $maximum_layer_width   \
	--time_limit $maximum_duration \
	--ram_limit $maxumum_ram_in_gb \
	--proba $proba                 \
	--seed  $the_seed              

  # TSPTW. During the experiments, we let the maximum width vary but used the
  # following fixed values for the other parameters.
  # 
  # $maximum_duration  = 600 (seconds)
  # $maximum_ram_in_gb = 2   
  # $seed              = 20211105
  # $proba             = 0.1
  #
  # Over the course of our experimental study, we performed two distinct run 
  # for each instance (the exact same has been done with choco-based solver).
  # Concretely, we ran the following command twice, once with the value of
  # $initial_solution as shown in `benchmarks/tsptw/intitial_solutions` and
  # once with $initial_solution being equal to the best known solution 
  # (as shown in `benchmarks/tsptw/best_known`.
  #
  psp2 solve -f $instance_file     \
  	--width $maximum_layer_width   \
	--time_limit $maximum_duration \
	--ram_limit $maxumum_ram_in_gb \
	--proba $proba                 \
	--seed  $the_seed              \
	--solution "$initial_solution"
  ```

* `solver-mip` comprises the source code of the state-of-the-art MIP models to
  solve the pigment sequencing problem. These models have been authored by 
  Wolsey et al and only slightly adapted by us in order to track the time before
  it finds its best known solution. These models have been programmed using the
  FICO-Xpress workbench.

* `solver-choco` contains the source code of the solver which we programmed in
  order to solve the traveling salesman problem with time windows using 
  constraint programming. As reported in our submitted paper, this is an highly
  effective model which managed to find new solutions improving over the 
  currently best known. This solver is written in Java and for simplicity of
  reproducing our experiments, we provide this solver as a maven project.
  A standalone executable jar for this solver can be produced with the following
  command: 
  ```
  # The resulting executable jar will be located at 
  # target/choco-tsptw-1.0-SNAPSHOT-jar-with-dependencies.jar
  mvn install 
  ```
  The resulting jar can be used to solve a TSPTW instance with the following
  command (for the sake of compactness, the jar has been renamed to tsptw.jar): 
  ```
  java -jar tsptw.jar            \
  	-f $instance_file            \
	-t $duration                 \
	-s "$initial_solution"       \
	-c $cost_of_initial_solution
  ```

* The folder `tsptw-initial` contains the source code of our implementation of
  Da Silva and Urrutia algorithm. That initial feasible solution solver was
  written in rust and must therefore be compiled with cargo.

## Benchmarks Folder

This folder contains two sub folders, one for each problem investigated during
our experimental study. In the `psp` subfolder, one will find:

* the file `best_known.csv` which maps the best known objective solution for
  all the instances used in the psp investigation.

* the folder `instances` which comprise all the psp instances that have been used
  for our experiment.

* the folder `instances-mip-processed` which comprise the exact same instances
  as the above, except the format of all these instances has been adapted and
  split accross multiple files for easier processing with the MIP models.

Similarly, the `tsptw` folder comprises the following sub folders:

* `best_known_solutions` which comprises one text file per tsptw benchmarks suite.
  These .txt files put in correspondance the id of an instance, its best known
  objective value as well as the solution leading to that best known objective.

* `intitial_solutions` which also contains one .txt file per benchmark suite.
  These .txt files establish a correspondance between the instance id, the 
  objective value of an intial solution (found w/ our implementation of Da Silva
  and Urrutia's algorithm) and the actual initial solution. The solutions in 
  these files are the ones that have been used to initialize the LNS process of
  both Choco and DD-LNS during our experimental study.

* `instances` which comprise all benchmark instances (in plaintext) grouped per
  benchmark suite. These instances are the same which can be found online at
  the following address: https://lopez-ibanez.eu/tsptw-instances. The best
  known solutions to these instances and their objective values can also be
  found on that site. We nevertheless reproduced them here for easier review.


## Results

The detailed result of all our experiments can be found in the `results` folder.

* `results/psp/mip` contains one file per mip model. mip1.txt corresponds to
  the resolution of PSP instances with the model known as PIG-A-1. Similarly, 
  mip2.txt corresponds to the results using the model known as PIG-A-2. And
  finally, mip3.txt corresponds to the results obtained with the model known
  as PIG-A-3.

* `results/psp/ddlns` contains files crresponding to the resolution of the 
  psp instances using our DD-LNS solver. The names of these files are structured
  as `lns.wXXX-tYYY.txt` where XXX denotes the maximum layer width allowed when
  solving the problem and YYY denotes the maximum time limit which was used
  (always 600).

* `results/tsptw/choco` contains two files: `improvements_choco_t600.txt` which
  contains the results of running the choco based CP model and initializing it
  with the best known solution. The file `results_choco_t600.txt` contains the
  results of running the choco based CP model on each instance, initializing the
  resolution with an intitial solution found using our implementation of Da Silva
  and Urrutia algorithm.

* `results/tsptw/ddlns` contains two sets of files: `improvements_wXXX_tYYY.txt`
  which contains the results of running our DD-LNS solver initialized with the
  best known solution (attempting to improve the best known solutions of 
  instances that have been open for years). And the files `results_wXXX_tYYY.txt`
  which comprise the results of trying to solve the tsptw instances based on
  an initial solution found with our implemntation of Da Silva and Urrutia.
  In both sets of files, the value of XXX indicates the maximum layer width
  allowed when compiling a decision diagram. And YYY indicates the maximum 
  allotted time (alwa 600 seconds).

