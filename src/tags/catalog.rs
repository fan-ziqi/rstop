//! Executable identities and the inspections useful for each role.
//! Names are exact basenames, never matches against arbitrary command arguments.
use super::{Confidence, Tag, push};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    Network,
    Files,
    Storage,
    Runtime,
    Communication,
    Memory,
    Cpu,
}
use Capability::*;

struct Role {
    key: &'static str,
    label: &'static str,
    names: &'static [&'static str],
    capabilities: &'static [Capability],
}
macro_rules! role {
    ($key:literal, $label:literal, [$($name:literal),* $(,)?], [$($cap:ident),* $(,)?]) => {
        Role { key: $key, label: $label, names: &[$($name),*], capabilities: &[$($cap),*] }
    };
}
static ROLES: &[Role] = &[
    role!(
        "web",
        "Web",
        [
            "nginx",
            "apache2",
            "httpd",
            "caddy",
            "traefik",
            "envoy",
            "haproxy",
            "gunicorn",
            "uvicorn",
            "uwsgi",
            "http.server",
            "php-fpm",
            "lighttpd",
            "frankenphp"
        ],
        [Network, Files, Runtime, Memory]
    ),
    role!(
        "database",
        "Database",
        [
            "postgres",
            "postgresql",
            "postmaster",
            "mysqld",
            "mariadbd",
            "mongod",
            "mongos",
            "cockroach",
            "clickhouse-server",
            "sqlservr",
            "influxd",
            "neo4j",
            "cassandra",
            "duckdb",
            "sqlite3",
            "surreal",
            "questdb"
        ],
        [Network, Files, Storage, Memory, Runtime]
    ),
    role!(
        "db-client",
        "DB client",
        [
            "psql",
            "mysql",
            "mariadb",
            "mongosh",
            "redis-cli",
            "valkey-cli",
            "pgcli",
            "mycli",
            "dbeaver"
        ],
        [Network, Files]
    ),
    role!(
        "cache",
        "Cache",
        [
            "redis-server",
            "redis",
            "valkey-server",
            "valkey",
            "memcached",
            "keydb-server",
            "varnishd"
        ],
        [Network, Memory, Storage]
    ),
    role!(
        "search",
        "Search",
        [
            "elasticsearch",
            "opensearch",
            "solr",
            "meilisearch",
            "typesense-server",
            "qdrant",
            "weaviate",
            "org.elasticsearch.bootstrap.elasticsearch",
            "org.opensearch.bootstrap.opensearch"
        ],
        [Network, Files, Storage, Memory, Runtime]
    ),
    role!(
        "middleware",
        "Messaging",
        [
            "zenohd",
            "zenoh-bridge-dds",
            "zenoh-bridge-ros2dds",
            "fastdds",
            "fast-discovery-server",
            "ddsrouter",
            "discovery-server",
            "mosquitto",
            "nats-server",
            "rabbitmq-server",
            "kafka-server-start.sh",
            "kafka.kafka",
            "redpanda",
            "pulsar",
            "emqx",
            "activemq",
            "beanstalkd"
        ],
        [Network, Files, Runtime, Memory]
    ),
    role!(
        "dds",
        "DDS",
        [
            "fastdds",
            "fast-discovery-server",
            "ddsrouter",
            "discovery-server",
            "cyclonedds",
            "ddsperf",
            "rtiroutingservice",
            "rtiddsspy"
        ],
        [Network, Runtime, Communication]
    ),
    role!(
        "zenoh",
        "Zenoh",
        ["zenohd", "zenoh-bridge-dds", "zenoh-bridge-ros2dds"],
        [Network, Runtime, Communication]
    ),
    role!(
        "ros",
        "ROS",
        [
            "ros2",
            "roscore",
            "rosmaster",
            "roslaunch",
            "rosout",
            "rosrun",
            "rostopic",
            "rosnode",
            "rosservice",
            "rosparam",
            "rviz",
            "rviz2",
            "rqt",
            "rqt_gui",
            "component_container",
            "component_container_mt",
            "component_container_isolated",
            "robot_state_publisher",
            "joint_state_publisher",
            "joint_state_publisher_gui",
            "move_group",
            "rosbag",
            "rosbag2",
            "ros2_control_node",
            "controller_manager",
            "slam_toolbox",
            "sync_slam_toolbox_node",
            "async_slam_toolbox_node",
            "map_server",
            "amcl",
            "bt_navigator",
            "planner_server",
            "controller_server",
            "behavior_server",
            "waypoint_follower",
            "velocity_smoother",
            "lifecycle_manager",
            "ekf_node",
            "ukf_node",
            "static_transform_publisher"
        ],
        [Network, Runtime, Communication, Files]
    ),
    role!(
        "robotics",
        "Robotics",
        [
            "ros2",
            "roscore",
            "rosmaster",
            "roslaunch",
            "rosout",
            "rviz",
            "rviz2",
            "component_container",
            "component_container_mt",
            "robot_state_publisher",
            "joint_state_publisher",
            "move_group",
            "rosbag",
            "rosbag2",
            "gzserver",
            "gzclient",
            "gazebo",
            "webots",
            "isaac-sim"
        ],
        [Network, Runtime, Communication, Files]
    ),
    role!(
        "simulation",
        "Simulation",
        [
            "gzserver",
            "gzclient",
            "gazebo",
            "webots",
            "isaac-sim",
            "mujoco",
            "coppeliasim",
            "carlaue4"
        ],
        [Runtime, Files, Memory, Cpu]
    ),
    role!(
        "model-server",
        "Model server",
        [
            "llama-server",
            "tritonserver",
            "text-generation-launcher",
            "text-generation-server",
            "sglang.launch_server",
            "vllm.entrypoints.openai.api_server"
        ],
        [Network, Runtime, Files, Memory]
    ),
    role!(
        "compute",
        "AI compute",
        [
            "ollama",
            "vllm",
            "torchrun",
            "llama-server",
            "tritonserver",
            "text-generation-launcher",
            "text-generation-server"
        ],
        [Network, Runtime, Files, Memory, Cpu]
    ),
    role!(
        "distributed-compute",
        "Distributed",
        [
            "torchrun",
            "mpirun",
            "mpiexec",
            "orted",
            "prterun",
            "hydra_pmi_proxy",
            "torch.distributed.run",
            "accelerate.commands.launch",
            "dask-scheduler",
            "dask-worker",
            "raylet",
            "gcs_server",
            "spark-submit",
            "org.apache.spark.deploy.sparksubmit"
        ],
        [Network, Runtime, Files, Memory, Cpu]
    ),
    role!(
        "scientific",
        "Scientific",
        [
            "matlab",
            "octave",
            "julia",
            "r",
            "rscript",
            "wolframkernel",
            "math",
            "gromacs",
            "gmx",
            "lmp",
            "lmp_mpi",
            "openfoam",
            "simplefoam"
        ],
        [Runtime, Files, Storage, Memory, Cpu]
    ),
    role!(
        "notebook",
        "Notebook",
        [
            "jupyter",
            "jupyter-lab",
            "jupyter-notebook",
            "jupyter-server",
            "jupyterlab",
            "jupyter_server",
            "notebook",
            "ipykernel_launcher",
            "ipython",
            "rstudio",
            "rstudio-server"
        ],
        [Network, Files, Runtime, Memory]
    ),
    role!(
        "build",
        "Build",
        [
            "gcc",
            "g++",
            "cc1",
            "cc1plus",
            "clang",
            "clang++",
            "rustc",
            "ninja",
            "make",
            "cmake",
            "ld",
            "lld",
            "ld.lld",
            "link",
            "msbuild",
            "csc",
            "javac",
            "gradle",
            "gradlew",
            "mvn",
            "mvnw",
            "bazel",
            "bazelisk",
            "meson",
            "scons",
            "xcodebuild",
            "swiftc",
            "nvcc",
            "hipcc",
            "ccache",
            "sccache",
            "webpack",
            "rollup",
            "esbuild",
            "vite"
        ],
        [Files, Storage, Memory, Cpu]
    ),
    role!(
        "testing",
        "Tests",
        [
            "pytest",
            "py.test",
            "unittest",
            "nose2",
            "ctest",
            "nextest",
            "jest",
            "vitest",
            "mocha",
            "ava",
            "playwright",
            "cypress",
            "phpunit",
            "rspec"
        ],
        [Files, Runtime, Memory, Cpu]
    ),
    role!(
        "lint",
        "Code check",
        [
            "ruff",
            "pylint",
            "mypy",
            "pyright",
            "basedpyright",
            "eslint",
            "biome",
            "shellcheck",
            "clang-tidy",
            "cppcheck",
            "golangci-lint",
            "rustfmt",
            "black",
            "prettier",
            "stylua"
        ],
        [Files, Cpu]
    ),
    role!(
        "packages",
        "Packages",
        [
            "apt",
            "apt-get",
            "dpkg",
            "dnf",
            "yum",
            "rpm",
            "pacman",
            "zypper",
            "brew",
            "flatpak",
            "snap",
            "snapd",
            "winget",
            "choco",
            "scoop",
            "nix",
            "nix-daemon",
            "conda",
            "mamba",
            "micromamba",
            "pip",
            "pip3",
            "pipx",
            "uv"
        ],
        [Network, Files, Storage]
    ),
    role!(
        "version-control",
        "Version control",
        [
            "git",
            "git-lfs",
            "svn",
            "hg",
            "fossil",
            "jj",
            "git-remote-https",
            "git-remote-http",
            "git-upload-pack",
            "git-receive-pack"
        ],
        [Network, Files, Storage]
    ),
    role!(
        "debugger",
        "Debugger",
        [
            "gdb",
            "lldb",
            "debugserver",
            "dlv",
            "cdb",
            "windbg",
            "debugpy",
            "debugpy.adapter"
        ],
        [Runtime, Files, Memory, Cpu]
    ),
    role!(
        "profiler",
        "Profiler",
        [
            "perf",
            "strace",
            "ltrace",
            "valgrind",
            "heaptrack",
            "py-spy",
            "nsys",
            "ncu",
            "bpftrace",
            "tcpdump",
            "dumpcap",
            "wireshark"
        ],
        [Runtime, Files, Memory, Cpu, Network]
    ),
    role!(
        "language-server",
        "Language server",
        [
            "rust-analyzer",
            "clangd",
            "gopls",
            "typescript-language-server",
            "pylsp",
            "pyright-langserver",
            "basedpyright-langserver",
            "lua-language-server",
            "jdtls",
            "sourcekit-lsp"
        ],
        [Files, Memory, Cpu]
    ),
    role!(
        "development",
        "Development",
        [
            "cargo", "go", "npm", "pnpm", "yarn", "corepack", "dotnet", "flutter", "dart", "mix",
            "bundle", "composer"
        ],
        [Network, Files, Storage, Runtime]
    ),
    role!(
        "editor",
        "Editor",
        [
            "code",
            "code-insiders",
            "codium",
            "cursor",
            "zed",
            "idea",
            "idea64",
            "pycharm",
            "clion",
            "rider",
            "webstorm",
            "goland",
            "studio",
            "eclipse",
            "nvim",
            "vim",
            "gvim",
            "emacs",
            "sublime_text",
            "notepad++",
            "devenv"
        ],
        [Network, Files, Memory, Runtime]
    ),
    role!(
        "ci",
        "CI worker",
        [
            "gitlab-runner",
            "buildkite-agent",
            "jenkins",
            "jenkins.war",
            "runner.listener",
            "runner.worker",
            "act_runner",
            "woodpecker-agent"
        ],
        [Network, Files, Storage, Runtime]
    ),
    role!(
        "container-runtime",
        "Containers",
        [
            "docker",
            "dockerd",
            "containerd",
            "containerd-shim",
            "containerd-shim-runc-v2",
            "podman",
            "conmon",
            "crun",
            "runc",
            "buildkitd",
            "buildctl",
            "nerdctl",
            "crio",
            "cri-o",
            "lxc-start",
            "incusd"
        ],
        [Network, Files, Storage, Runtime]
    ),
    role!(
        "orchestration",
        "Kubernetes",
        [
            "kubectl",
            "kubelet",
            "kube-apiserver",
            "kube-controller-manager",
            "kube-scheduler",
            "kube-proxy",
            "k3s",
            "rke2",
            "helm",
            "minikube",
            "kind",
            "k9s"
        ],
        [Network, Files, Storage, Runtime]
    ),
    role!(
        "virtualization",
        "VM",
        [
            "qemu-system-x86_64",
            "qemu-system-aarch64",
            "qemu-kvm",
            "kvm",
            "libvirtd",
            "virtqemud",
            "virtualbox",
            "virtualboxvm",
            "vboxheadless",
            "vmware-vmx",
            "vmware",
            "vmwp",
            "vmmem",
            "vmmemwsl",
            "wslhost",
            "firecracker",
            "cloud-hypervisor",
            "multipassd"
        ],
        [Network, Files, Storage, Memory, Cpu]
    ),
    role!(
        "observability",
        "Monitoring",
        [
            "prometheus",
            "grafana",
            "grafana-server",
            "node_exporter",
            "process-exporter",
            "alertmanager",
            "loki",
            "promtail",
            "vector",
            "fluent-bit",
            "fluentd",
            "otelcol",
            "otelcol-contrib",
            "telegraf",
            "collectd",
            "netdata",
            "zabbix_agentd",
            "datadog-agent"
        ],
        [Network, Files, Storage, Memory]
    ),
    role!(
        "remote",
        "Remote access",
        [
            "ssh",
            "sshd",
            "scp",
            "sftp",
            "sftp-server",
            "mosh",
            "mosh-server",
            "xrdp",
            "xrdp-sesman",
            "remmina",
            "xfreerdp",
            "wfreerdp",
            "mstsc",
            "x11vnc",
            "tigervnc",
            "xtigervnc",
            "wayvnc",
            "rustdesk",
            "anydesk",
            "teamviewer",
            "sunshine",
            "moonlight"
        ],
        [Network, Files, Runtime]
    ),
    role!(
        "vpn",
        "VPN / tunnel",
        [
            "openvpn",
            "wireguard-go",
            "wg-quick",
            "tailscaled",
            "tailscale",
            "zerotier-one",
            "nebula",
            "tincd",
            "frpc",
            "frps",
            "cloudflared",
            "ngrok",
            "chisel",
            "boringtun"
        ],
        [Network, Files]
    ),
    role!(
        "proxy",
        "Proxy",
        [
            "mihomo",
            "clash",
            "clash-meta",
            "sing-box",
            "xray",
            "v2ray",
            "ss-local",
            "ss-server",
            "privoxy",
            "squid",
            "redsocks",
            "tor"
        ],
        [Network, Files]
    ),
    role!(
        "dns",
        "DNS",
        [
            "named",
            "bind9",
            "unbound",
            "dnsmasq",
            "coredns",
            "systemd-resolved",
            "dnscrypt-proxy",
            "adguardhome",
            "pihole-ftl",
            "smartdns",
            "stubby"
        ],
        [Network, Files]
    ),
    role!(
        "network",
        "Network",
        [
            "curl",
            "wget",
            "wget2",
            "aria2c",
            "transmission-daemon",
            "transmission-gtk",
            "qbittorrent",
            "qbittorrent-nox",
            "deluged",
            "iperf",
            "iperf3",
            "netperf",
            "ping",
            "ping6",
            "traceroute",
            "mtr",
            "dig",
            "nslookup",
            "nmap",
            "socat",
            "nc",
            "ncat",
            "netcat"
        ],
        [Network, Files]
    ),
    role!(
        "sync",
        "Sync",
        [
            "rsync",
            "rclone",
            "syncthing",
            "unison",
            "nextcloud",
            "owncloud",
            "dropbox",
            "onedrive",
            "seafile-applet",
            "lsyncd"
        ],
        [Network, Files, Storage]
    ),
    role!(
        "backup",
        "Backup",
        [
            "restic",
            "borg",
            "borgmatic",
            "kopia",
            "duplicity",
            "duplicati",
            "bup",
            "urbackupclientbackend",
            "veeamagent"
        ],
        [Network, Files, Storage]
    ),
    role!(
        "file-sharing",
        "File sharing",
        [
            "smbd",
            "nmbd",
            "samba",
            "nfsd",
            "rpc.mountd",
            "rpcbind",
            "vsftpd",
            "proftpd",
            "pure-ftpd",
            "minio",
            "garage",
            "radosgw"
        ],
        [Network, Files, Storage]
    ),
    role!(
        "archive",
        "Archive",
        [
            "tar", "gzip", "gunzip", "pigz", "bzip2", "xz", "zstd", "zip", "unzip", "7z", "7zz",
            "rar", "unrar", "bsdtar"
        ],
        [Files, Storage, Cpu]
    ),
    role!(
        "media",
        "Media",
        [
            "ffmpeg",
            "ffprobe",
            "ffplay",
            "vlc",
            "mpv",
            "obs",
            "obs64",
            "obs-studio",
            "handbrakecli",
            "handbrake",
            "blender",
            "gimp",
            "krita",
            "inkscape",
            "darktable",
            "rawtherapee",
            "davinci-resolve",
            "resolve",
            "audacity",
            "ardour",
            "kdenlive",
            "shotcut",
            "olive-editor",
            "imagemagick",
            "magick"
        ],
        [Files, Storage, Runtime, Memory, Cpu]
    ),
    role!(
        "media-convert",
        "Media convert",
        ["ffmpeg", "handbrakecli", "handbrake", "magick"],
        [Files, Storage, Cpu]
    ),
    role!(
        "media-player",
        "Playback",
        [
            "ffplay",
            "vlc",
            "mpv",
            "totem",
            "rhythmbox",
            "spotify",
            "music",
            "itunes"
        ],
        [Network, Files]
    ),
    role!(
        "streaming",
        "Capture / stream",
        [
            "obs",
            "obs64",
            "obs-studio",
            "wf-recorder",
            "gpu-screen-recorder"
        ],
        [Network, Files, Storage]
    ),
    role!(
        "render",
        "3D / render",
        [
            "blender",
            "maya",
            "mayabatch",
            "houdini",
            "hython",
            "husk",
            "cinema4d",
            "unrealengine",
            "unrealeditor",
            "unity",
            "godot"
        ],
        [Files, Runtime, Memory, Cpu]
    ),
    role!(
        "browser",
        "Browser",
        [
            "firefox",
            "firefox-esr",
            "chrome",
            "chromium",
            "chromium-browser",
            "google-chrome",
            "google-chrome-stable",
            "msedge",
            "microsoft-edge",
            "brave",
            "brave-browser",
            "vivaldi",
            "opera",
            "safari",
            "webkitwebprocess",
            "waterfox",
            "librewolf"
        ],
        [Network, Files, Memory, Runtime]
    ),
    role!(
        "communication",
        "Calls / chat",
        [
            "discord",
            "slack",
            "teams",
            "ms-teams",
            "zoom",
            "zoom.us",
            "telegram-desktop",
            "signal-desktop",
            "element-desktop",
            "wechat",
            "weixin",
            "qq",
            "thunderbird",
            "outlook"
        ],
        [Network, Files, Memory]
    ),
    role!(
        "office",
        "Office",
        [
            "soffice",
            "soffice.bin",
            "libreoffice",
            "onlyoffice",
            "desktopeditors",
            "wps",
            "wpp",
            "et",
            "winword",
            "excel",
            "powerpnt",
            "onenote",
            "evince",
            "okular",
            "zathura",
            "sumatrapdf",
            "acrobat",
            "acrord32"
        ],
        [Files, Memory]
    ),
    role!(
        "games",
        "Games / launcher",
        [
            "steam",
            "steamwebhelper",
            "steamservice",
            "lutris",
            "heroic",
            "epicgameslauncher",
            "battle.net",
            "gamescope",
            "wine",
            "wine64",
            "wineserver",
            "proton",
            "prismlauncher",
            "minecraft-launcher"
        ],
        [Network, Files, Memory, Runtime]
    ),
    role!(
        "desktop",
        "Desktop / audio",
        [
            "gnome-shell",
            "kwin_wayland",
            "kwin_x11",
            "plasmashell",
            "sway",
            "hyprland",
            "weston",
            "xorg",
            "xwayland",
            "windowserver",
            "dwm",
            "explorer",
            "pipewire",
            "pipewire-pulse",
            "wireplumber",
            "pulseaudio",
            "coreaudiod",
            "audiodg"
        ],
        [Runtime, Files, Memory]
    ),
    role!(
        "terminal",
        "Terminal",
        [
            "alacritty",
            "kitty",
            "wezterm",
            "wezterm-gui",
            "gnome-terminal-server",
            "konsole",
            "foot",
            "xterm",
            "tmux",
            "screen",
            "windowsterminal",
            "conhost",
            "iterm2"
        ],
        [Files, Runtime]
    ),
    role!(
        "scheduler",
        "Scheduled jobs",
        [
            "cron",
            "crond",
            "anacron",
            "atd",
            "systemd",
            "launchd",
            "taskeng",
            "taskhostw",
            "slurmd",
            "slurmctld",
            "srun",
            "sbatch"
        ],
        [Files, Cpu]
    ),
    role!(
        "indexer",
        "File indexing",
        [
            "updatedb",
            "tracker-miner-fs",
            "tracker-miner-fs-3",
            "tracker-extract",
            "baloo_file",
            "baloo_file_extractor",
            "mds",
            "mdworker",
            "searchindexer",
            "locate",
            "plocate"
        ],
        [Files, Storage, Memory, Cpu]
    ),
    role!(
        "automation",
        "Automation",
        [
            "ansible",
            "ansible-playbook",
            "terraform",
            "tofu",
            "pulumi",
            "packer",
            "puppet",
            "chef-client",
            "salt-minion",
            "salt-master"
        ],
        [Network, Files, Storage, Runtime]
    ),
    role!(
        "cloud-cli",
        "Cloud CLI",
        [
            "aws", "az", "gcloud", "gsutil", "doctl", "hcloud", "aliyun", "ossutil", "coscli"
        ],
        [Network, Files]
    ),
    role!(
        "job-worker",
        "Job worker",
        [
            "celery",
            "rq",
            "rqworker",
            "dramatiq",
            "airflow",
            "dagster",
            "prefect",
            "temporal-server",
            "sidekiq",
            "resque"
        ],
        [Network, Files, Storage, Memory, Cpu, Runtime]
    ),
    role!(
        "security",
        "Security scan",
        [
            "clamd",
            "clamscan",
            "clamdscan",
            "freshclam",
            "msmpeng",
            "mrt",
            "rkhunter",
            "aide",
            "osqueryd",
            "wazuh-agentd",
            "falco"
        ],
        [Files, Storage, Cpu]
    ),
    // Intent/resource/state labels share the same action policy as software roles.
    role!(
        "training",
        "Training",
        [],
        [Files, Storage, Memory, Cpu, Runtime]
    ),
    role!("gpu-active", "GPU active", [], [Memory, Runtime]),
    role!("gpu-memory", "GPU memory", [], [Memory, Runtime]),
    role!("cpu-cores", "CPU >=1 core", [], [Cpu]),
    role!("memory-heavy", "RAM >=1 GiB", [], [Memory]),
    role!("disk-io", "Disk IO", [], [Files, Storage]),
    role!("network-io", "Network IO", [], [Network]),
    role!("listener", "Listener", [], [Network]),
    role!("connecting", "Connecting", [], [Network]),
    role!("socket-queue", "Socket queue", [], [Network]),
    role!("zombie", "Zombie", [], [Cpu]),
    role!("stopped", "Stopped", [], [Cpu]),
    role!("failed", "Failed", [], [Files]),
    role!("restarting", "Restarting", [], [Files, Memory]),
    role!("unhealthy", "Unhealthy", [], [Network, Memory]),
];

pub(super) fn recognize(name: &str, source: &str, tags: &mut Vec<Tag>) {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<&'static str, Vec<&'static Role>>> =
        std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let mut index = std::collections::HashMap::<_, Vec<_>>::new();
        for role in ROLES {
            for &name in role.names {
                index.entry(name).or_default().push(role);
            }
        }
        index
    });
    for role in index.get(name).into_iter().flatten() {
        push(
            tags,
            role.key,
            role.label,
            Confidence::Observed,
            format!("{source}: {name} (recognized executable identity)"),
        );
    }
}

pub fn supports(tags: &[Tag], capability: Capability) -> bool {
    tags.iter().any(|tag| {
        ROLES
            .iter()
            .any(|r| r.key == tag.key && r.capabilities.contains(&capability))
    })
}
