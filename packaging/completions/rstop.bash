_rstop() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="rstop"
                ;;
            rstop,completions)
                cmd="rstop__subcmd__completions"
                ;;
            rstop,config)
                cmd="rstop__subcmd__config"
                ;;
            rstop,doctor)
                cmd="rstop__subcmd__doctor"
                ;;
            rstop,export)
                cmd="rstop__subcmd__export"
                ;;
            rstop,gpu)
                cmd="rstop__subcmd__gpu"
                ;;
            rstop,help)
                cmd="rstop__subcmd__help"
                ;;
            rstop,man)
                cmd="rstop__subcmd__man"
                ;;
            rstop,monitor)
                cmd="rstop__subcmd__monitor"
                ;;
            rstop,network)
                cmd="rstop__subcmd__network"
                ;;
            rstop,record)
                cmd="rstop__subcmd__record"
                ;;
            rstop,replay)
                cmd="rstop__subcmd__replay"
                ;;
            rstop,sample)
                cmd="rstop__subcmd__sample"
                ;;
            rstop,snapshot)
                cmd="rstop__subcmd__snapshot"
                ;;
            rstop,update)
                cmd="rstop__subcmd__update"
                ;;
            rstop__subcmd__config,help)
                cmd="rstop__subcmd__config__subcmd__help"
                ;;
            rstop__subcmd__config,path)
                cmd="rstop__subcmd__config__subcmd__path"
                ;;
            rstop__subcmd__config,write)
                cmd="rstop__subcmd__config__subcmd__write"
                ;;
            rstop__subcmd__config__subcmd__help,help)
                cmd="rstop__subcmd__config__subcmd__help__subcmd__help"
                ;;
            rstop__subcmd__config__subcmd__help,path)
                cmd="rstop__subcmd__config__subcmd__help__subcmd__path"
                ;;
            rstop__subcmd__config__subcmd__help,write)
                cmd="rstop__subcmd__config__subcmd__help__subcmd__write"
                ;;
            rstop__subcmd__gpu,help)
                cmd="rstop__subcmd__gpu__subcmd__help"
                ;;
            rstop__subcmd__gpu,select)
                cmd="rstop__subcmd__gpu__subcmd__select"
                ;;
            rstop__subcmd__gpu__subcmd__help,help)
                cmd="rstop__subcmd__gpu__subcmd__help__subcmd__help"
                ;;
            rstop__subcmd__gpu__subcmd__help,select)
                cmd="rstop__subcmd__gpu__subcmd__help__subcmd__select"
                ;;
            rstop__subcmd__help,completions)
                cmd="rstop__subcmd__help__subcmd__completions"
                ;;
            rstop__subcmd__help,config)
                cmd="rstop__subcmd__help__subcmd__config"
                ;;
            rstop__subcmd__help,doctor)
                cmd="rstop__subcmd__help__subcmd__doctor"
                ;;
            rstop__subcmd__help,export)
                cmd="rstop__subcmd__help__subcmd__export"
                ;;
            rstop__subcmd__help,gpu)
                cmd="rstop__subcmd__help__subcmd__gpu"
                ;;
            rstop__subcmd__help,help)
                cmd="rstop__subcmd__help__subcmd__help"
                ;;
            rstop__subcmd__help,man)
                cmd="rstop__subcmd__help__subcmd__man"
                ;;
            rstop__subcmd__help,monitor)
                cmd="rstop__subcmd__help__subcmd__monitor"
                ;;
            rstop__subcmd__help,network)
                cmd="rstop__subcmd__help__subcmd__network"
                ;;
            rstop__subcmd__help,record)
                cmd="rstop__subcmd__help__subcmd__record"
                ;;
            rstop__subcmd__help,replay)
                cmd="rstop__subcmd__help__subcmd__replay"
                ;;
            rstop__subcmd__help,sample)
                cmd="rstop__subcmd__help__subcmd__sample"
                ;;
            rstop__subcmd__help,snapshot)
                cmd="rstop__subcmd__help__subcmd__snapshot"
                ;;
            rstop__subcmd__help,update)
                cmd="rstop__subcmd__help__subcmd__update"
                ;;
            rstop__subcmd__help__subcmd__config,path)
                cmd="rstop__subcmd__help__subcmd__config__subcmd__path"
                ;;
            rstop__subcmd__help__subcmd__config,write)
                cmd="rstop__subcmd__help__subcmd__config__subcmd__write"
                ;;
            rstop__subcmd__help__subcmd__gpu,select)
                cmd="rstop__subcmd__help__subcmd__gpu__subcmd__select"
                ;;
            rstop__subcmd__help__subcmd__network,capture)
                cmd="rstop__subcmd__help__subcmd__network__subcmd__capture"
                ;;
            rstop__subcmd__help__subcmd__network,check)
                cmd="rstop__subcmd__help__subcmd__network__subcmd__check"
                ;;
            rstop__subcmd__help__subcmd__network,connections)
                cmd="rstop__subcmd__help__subcmd__network__subcmd__connections"
                ;;
            rstop__subcmd__help__subcmd__network,read)
                cmd="rstop__subcmd__help__subcmd__network__subcmd__read"
                ;;
            rstop__subcmd__network,capture)
                cmd="rstop__subcmd__network__subcmd__capture"
                ;;
            rstop__subcmd__network,check)
                cmd="rstop__subcmd__network__subcmd__check"
                ;;
            rstop__subcmd__network,connections)
                cmd="rstop__subcmd__network__subcmd__connections"
                ;;
            rstop__subcmd__network,help)
                cmd="rstop__subcmd__network__subcmd__help"
                ;;
            rstop__subcmd__network,read)
                cmd="rstop__subcmd__network__subcmd__read"
                ;;
            rstop__subcmd__network__subcmd__help,capture)
                cmd="rstop__subcmd__network__subcmd__help__subcmd__capture"
                ;;
            rstop__subcmd__network__subcmd__help,check)
                cmd="rstop__subcmd__network__subcmd__help__subcmd__check"
                ;;
            rstop__subcmd__network__subcmd__help,connections)
                cmd="rstop__subcmd__network__subcmd__help__subcmd__connections"
                ;;
            rstop__subcmd__network__subcmd__help,help)
                cmd="rstop__subcmd__network__subcmd__help__subcmd__help"
                ;;
            rstop__subcmd__network__subcmd__help,read)
                cmd="rstop__subcmd__network__subcmd__help__subcmd__read"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        rstop)
            opts="-i -h -V --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help --version monitor snapshot sample record replay export gpu network doctor config completions man update help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__completions)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help bash elvish fish powershell zsh"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help path write help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config__subcmd__help)
            opts="path write help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config__subcmd__help__subcmd__path)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config__subcmd__help__subcmd__write)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config__subcmd__path)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__config__subcmd__write)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__doctor)
            opts="-i -h --input --json --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --input)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__export)
            opts="-i -h --input --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help csv svg"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --input)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__gpu)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help select help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__gpu__subcmd__help)
            opts="select help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__gpu__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__gpu__subcmd__help__subcmd__select)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__gpu__subcmd__select)
            opts="-i -h --count --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help)
            opts="monitor snapshot sample record replay export gpu network doctor config completions man update help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__completions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__config)
            opts="path write"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__config__subcmd__path)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__config__subcmd__write)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__gpu)
            opts="select"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__gpu__subcmd__select)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__man)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__monitor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__network)
            opts="connections check capture read"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__network__subcmd__capture)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__network__subcmd__check)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__network__subcmd__connections)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__network__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__record)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__replay)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__sample)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__man)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__monitor)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help connections check capture read help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__capture)
            opts="-i -h --interface --filter --seconds --packets --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interface)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --seconds)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --packets)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__check)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__connections)
            opts="-i -h --filter --json --sort --reverse --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -W "traffic process state remote pid protocol local download upload received sent rtt receive-queue send-queue retransmits total-bytes local-port remote-port" -- "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__help)
            opts="connections check capture read help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__help__subcmd__capture)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__help__subcmd__check)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__help__subcmd__connections)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__help__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__network__subcmd__read)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__record)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__replay)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__sample)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__snapshot)
            opts="-i -h --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        rstop__subcmd__update)
            opts="-i -h --check --interval --theme --view --gpu-panel --config --no-gpu --snapshot --samples --export-svg --width --height --write-config --record --csv --replay --ascii --no-color --threads --process-network --read-only --full-sampling --layout --select-gpu --min-free-mib --min-total-mib --max-gpu-utilization --max-memory-utilization --no-config-exec --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --theme)
                    COMPREPLY=($(compgen -W "midnight nord ember black white graphite ocean forest sand" -- "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -W "overview processes gpu storage network system containers services explorer history events" -- "${cur}"))
                    return 0
                    ;;
                --gpu-panel)
                    COMPREPLY=($(compgen -W "summary history device" -- "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --samples)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export-svg)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --height)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --record)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --csv)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --layout)
                    COMPREPLY=($(compgen -W "balanced compute storage minimal" -- "${cur}"))
                    return 0
                    ;;
                --select-gpu)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-free-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-total-mib)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-gpu-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-memory-utilization)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _rstop -o nosort -o bashdefault -o default rstop
else
    complete -F _rstop -o bashdefault -o default rstop
fi
