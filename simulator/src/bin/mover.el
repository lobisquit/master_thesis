(defun copy-project ()
  (copy-file
   "/home/enrico/Archivi/master_thesis/data/aachen_net/topology.txt"
   "/ssh:lovisott@login.dei.unipd.it:/home/lovisott/topology.txt" t)

  (copy-file
   "/home/enrico/Archivi/master_thesis/simulator/target/release/aachen_net"
   "/ssh:lovisott@login.dei.unipd.it:/home/lovisott/aachen_net" t)

  (copy-file
   "/home/enrico/Archivi/master_thesis/simulator/src/bin/aachen_net.job"
   "/ssh:lovisott@login.dei.unipd.it:/home/lovisott/aachen_net.job" t))

(copy-project)
