set term pdf #size 30cm,20cm
set output "time-to-gap.pdf"

set datafile separator ';'

#set title "Cumulated number of instances, solved to optimality over time"
set ylabel "# instances"
set xlabel "time (seconds)"
set key right bottom #center
set xrange[0:600]

plot \
	"processed/mip1.gap.data"    		using 1:0 lt 1 lc 1  title "PIG-A-1", \
	"processed/mip2.gap.data"    		using 1:0 lt 1 lc 2  title "PIG-A-2", \
	"processed/mip3.gap.data"    		using 1:0 lt 1 lc 3  title "PIG-A-3", \
	"processed/lns-w10.gap.data"        using 1:0 lt 2 lc 7  title "LNS+DD (w10)", \
	"processed/lns-w100.gap.data"       using 1:0 lt 2 lc 4  title "LNS+DD (w100)", \
	"processed/lns-w1000.gap.data"      using 1:0 lt 2 lc 6  title "LNS+DD (w1000)", \
	#"processed/lns-w10000.gap.data"     using 1:0 lt 2 lc 7  title "LNS+DD (w10000)", \
	#;
