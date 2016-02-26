Vagrant.configure("2") do |config|

  config.vm.box = "ubuntu/trusty64"
  config.vm.hostname = "factotum"
  config.ssh.forward_agent = true

  # Forward guest port 3000 to host port 3000 (for command API)
  config.vm.network "forwarded_port", guest: 3000, host: 3000

  config.vm.provider :virtualbox do |vb|
    vb.name = Dir.pwd().split("/")[-1] + "-" + Time.now.to_f.to_i.to_s
    vb.customize ["modifyvm", :id, "--natdnshostresolver1", "on"]
    vb.customize [ "guestproperty", "set", :id, "--timesync-threshold", 10000 ]
    # Rust isn't very memory hungry 
    vb.memory = 2048
  end

  config.push.define "local-exec" do |push|
    push.script = "vagrant/push.bash"
  end

  config.vm.provision :shell do |sh|
    sh.path = "vagrant/up.bash"
  end

end
