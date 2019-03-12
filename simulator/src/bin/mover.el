(defun copy-project ()
  (defvar remote-dir "/ssh:lovisott@login.dei.unipd.it:/home/lovisott/aachen_net/")
  (mkdir remote-dir t)

  (mkdir (concat remote-dir "results/") t)

  (copy-file
   "/home/enrico/Archivi/master_thesis/data/aachen_net/topology.txt"
   (concat remote-dir "topology.txt") t)

  (copy-file
   "/home/enrico/Archivi/master_thesis/simulator/target/release/aachen_net"
   (concat remote-dir "aachen_net") t)

  (copy-file
   "/home/enrico/Archivi/master_thesis/simulator/src/bin/aachen_net.job"
   (concat remote-dir "aachen_net.job") t))

(copy-project)
