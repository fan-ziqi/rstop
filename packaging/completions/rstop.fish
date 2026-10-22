# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_rstop_global_optspecs
    string join \n i/interval= theme= view= gpu-panel= config= no-gpu snapshot samples= export-svg= width= height= write-config record= csv= replay= ascii no-color threads process-network read-only full-sampling layout= select-gpu= min-free-mib= min-total-mib= max-gpu-utilization= max-memory-utilization= no-config-exec h/help V/version
end

function __fish_rstop_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_rstop_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_rstop_using_subcommand
    set -l cmd (__fish_rstop_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c rstop -n "__fish_rstop_needs_command" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_needs_command" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_needs_command" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_needs_command" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_needs_command" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_needs_command" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_needs_command" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_needs_command" -l width -r
complete -c rstop -n "__fish_rstop_needs_command" -l height -r
complete -c rstop -n "__fish_rstop_needs_command" -l record -r -F
complete -c rstop -n "__fish_rstop_needs_command" -l csv -r -F
complete -c rstop -n "__fish_rstop_needs_command" -l replay -r -F
complete -c rstop -n "__fish_rstop_needs_command" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_needs_command" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_needs_command" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_needs_command" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_needs_command" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_needs_command" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_needs_command" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_needs_command" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_needs_command" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_needs_command" -l ascii
complete -c rstop -n "__fish_rstop_needs_command" -l no-color
complete -c rstop -n "__fish_rstop_needs_command" -l threads
complete -c rstop -n "__fish_rstop_needs_command" -l process-network
complete -c rstop -n "__fish_rstop_needs_command" -l read-only
complete -c rstop -n "__fish_rstop_needs_command" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_needs_command" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_needs_command" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_needs_command" -s V -l version -d 'Print version'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "monitor" -d 'Open the interactive system monitor (also the default command)'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "snapshot" -d 'Print one JSON snapshot, including process command lines'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "sample" -d 'Print newline-delimited JSON samples (ten by default)'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "record" -d 'Record telemetry while monitoring; --samples selects headless recording'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "replay" -d 'Replay a local recording; --samples selects JSON output'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "export" -d 'Export live telemetry or a recording to CSV or SVG'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "gpu" -d 'Open the GPU view or select CUDA devices'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "network" -d 'Open connection monitoring, test a target, or capture packets'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "doctor" -d 'Explain resource problems, recent trends and useful next steps'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "config" -d 'Print the configuration path or write current preferences'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "completions" -d 'Generate shell completions without reading configuration'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "man" -d 'Print the manual page without reading configuration'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "update" -d 'Update this installer-managed copy of rstop'
complete -c rstop -n "__fish_rstop_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand monitor" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand snapshot" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand sample" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand sample" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand sample" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand record" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand record" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand record" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand record" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand record" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand record" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand record" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand record" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand record" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand record" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand record" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand record" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand record" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand record" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand record" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand record" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand record" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand record" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand record" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand record" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand record" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand replay" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand replay" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand replay" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand export" -l input -r -F
complete -c rstop -n "__fish_rstop_using_subcommand export" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand export" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand export" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand export" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand export" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand export" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand export" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand export" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand export" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand export" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand export" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand export" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand export" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand export" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand export" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand export" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand export" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand export" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand export" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand export" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand export" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -f -a "select" -d 'Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and not __fish_seen_subcommand_from select help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l count -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from select" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from help" -f -a "select" -d 'Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs'
complete -c rstop -n "__fish_rstop_using_subcommand gpu; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -f -a "connections" -d 'List sockets and their owners; samples TCP counters twice for rates'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -f -a "check" -d 'Check an HTTP(S) URL or TCP host:port, with bounded timeouts'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -f -a "capture" -d 'Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -f -a "read" -d 'Explain and preview the first 60 headers in a local PCAP'
complete -c rstop -n "__fish_rstop_using_subcommand network; and not __fish_seen_subcommand_from connections check capture read help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l filter -d 'AND terms: port:443 process:python state:LISTEN pid:42 !proto:udp' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l sort -d 'Choose a network field; missing values always sort last' -r -f -a "traffic\t''
process\t''
state\t''
remote\t''
pid\t''
protocol\t''
local\t''
download\t''
upload\t''
received\t''
sent\t''
rtt\t''
receive-queue\t''
send-queue\t''
retransmits\t''
total-bytes\t''
local-port\t''
remote-port\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l json
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l reverse -d 'Reverse the chosen field\'s default direction'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from connections" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from check" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l interface -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l filter -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l seconds -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l packets -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from capture" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from read" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "connections" -d 'List sockets and their owners; samples TCP counters twice for rates'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "check" -d 'Check an HTTP(S) URL or TCP host:port, with bounded timeouts'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "capture" -d 'Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "read" -d 'Explain and preview the first 60 headers in a local PCAP'
complete -c rstop -n "__fish_rstop_using_subcommand network; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l input -d 'Diagnose an existing recording without inspecting this machine' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l json -d 'Print structured findings and the explanation as JSON'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -f -a "path" -d 'Print the configuration path'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -f -a "write" -d 'Save preferences to the configuration path'
complete -c rstop -n "__fish_rstop_using_subcommand config; and not __fish_seen_subcommand_from path write help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from path" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from write" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "path" -d 'Print the configuration path'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "write" -d 'Save preferences to the configuration path'
complete -c rstop -n "__fish_rstop_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand completions" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand completions" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand completions" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand man" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand man" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand man" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand man" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand man" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand man" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand man" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand man" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand man" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand man" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand man" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand man" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand man" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand man" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand man" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand man" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand man" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand man" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand man" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand man" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand man" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand update" -s i -l interval -d 'Sampling interval in milliseconds (250..=10000)' -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l theme -d 'Color theme (also adjustable in F2 settings)' -r -f -a "midnight\t''
nord\t''
ember\t''
black\t''
white\t''
graphite\t''
ocean\t''
forest\t''
sand\t''"
complete -c rstop -n "__fish_rstop_using_subcommand update" -l view -r -f -a "overview\t''
processes\t''
gpu\t''
storage\t''
network\t''
system\t''
containers\t''
services\t''
explorer\t''
history\t''
events\t''"
complete -c rstop -n "__fish_rstop_using_subcommand update" -l gpu-panel -d 'Initial GPU display; device opens Device info (history aliases summary)' -r -f -a "summary\t''
history\t''
device\t''"
complete -c rstop -n "__fish_rstop_using_subcommand update" -l config -d 'Alternate configuration file' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand update" -l samples -d 'Number of samples; emits newline-delimited JSON, or fills SVG graph history' -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l export-svg -d 'Render real collected data to an SVG, without opening a terminal' -r -F
complete -c rstop -n "__fish_rstop_using_subcommand update" -l width -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l height -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l record -r -F
complete -c rstop -n "__fish_rstop_using_subcommand update" -l csv -r -F
complete -c rstop -n "__fish_rstop_using_subcommand update" -l replay -r -F
complete -c rstop -n "__fish_rstop_using_subcommand update" -l layout -r -f -a "balanced\t''
compute\t''
storage\t''
minimal\t''"
complete -c rstop -n "__fish_rstop_using_subcommand update" -l select-gpu -d 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit' -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l min-free-mib -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l min-total-mib -d 'Minimum total VRAM for GPU selection' -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l max-gpu-utilization -d 'GPU selection ceiling; devices with unknown utilization are excluded' -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l max-memory-utilization -d 'GPU selection memory-controller utilization ceiling' -r
complete -c rstop -n "__fish_rstop_using_subcommand update" -l check -d 'Check availability without installing'
complete -c rstop -n "__fish_rstop_using_subcommand update" -l no-gpu -d 'Disable GPU driver queries'
complete -c rstop -n "__fish_rstop_using_subcommand update" -l snapshot -d 'Print a JSON snapshot, then exit (includes process commands)'
complete -c rstop -n "__fish_rstop_using_subcommand update" -l write-config -d 'Write preferences to the configuration file, then exit'
complete -c rstop -n "__fish_rstop_using_subcommand update" -l ascii
complete -c rstop -n "__fish_rstop_using_subcommand update" -l no-color
complete -c rstop -n "__fish_rstop_using_subcommand update" -l threads
complete -c rstop -n "__fish_rstop_using_subcommand update" -l process-network
complete -c rstop -n "__fish_rstop_using_subcommand update" -l read-only
complete -c rstop -n "__fish_rstop_using_subcommand update" -l full-sampling -d 'Preload all pages and keep their background sampling active'
complete -c rstop -n "__fish_rstop_using_subcommand update" -l no-config-exec -d 'Disable configured probes and alert commands'
complete -c rstop -n "__fish_rstop_using_subcommand update" -s h -l help -d 'Print help'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "monitor" -d 'Open the interactive system monitor (also the default command)'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "snapshot" -d 'Print one JSON snapshot, including process command lines'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "sample" -d 'Print newline-delimited JSON samples (ten by default)'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "record" -d 'Record telemetry while monitoring; --samples selects headless recording'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "replay" -d 'Replay a local recording; --samples selects JSON output'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "export" -d 'Export live telemetry or a recording to CSV or SVG'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "gpu" -d 'Open the GPU view or select CUDA devices'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "network" -d 'Open connection monitoring, test a target, or capture packets'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "doctor" -d 'Explain resource problems, recent trends and useful next steps'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "config" -d 'Print the configuration path or write current preferences'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "completions" -d 'Generate shell completions without reading configuration'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "man" -d 'Print the manual page without reading configuration'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "update" -d 'Update this installer-managed copy of rstop'
complete -c rstop -n "__fish_rstop_using_subcommand help; and not __fish_seen_subcommand_from monitor snapshot sample record replay export gpu network doctor config completions man update help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from gpu" -f -a "select" -d 'Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "connections" -d 'List sockets and their owners; samples TCP counters twice for rates'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "check" -d 'Check an HTTP(S) URL or TCP host:port, with bounded timeouts'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "capture" -d 'Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from network" -f -a "read" -d 'Explain and preview the first 60 headers in a local PCAP'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "path" -d 'Print the configuration path'
complete -c rstop -n "__fish_rstop_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "write" -d 'Save preferences to the configuration path'
