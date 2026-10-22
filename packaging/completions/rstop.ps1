
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'rstop' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'rstop'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'rstop' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('monitor', 'monitor', [CompletionResultType]::ParameterValue, 'Open the interactive system monitor (also the default command)')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Print one JSON snapshot, including process command lines')
            [CompletionResult]::new('sample', 'sample', [CompletionResultType]::ParameterValue, 'Print newline-delimited JSON samples (ten by default)')
            [CompletionResult]::new('record', 'record', [CompletionResultType]::ParameterValue, 'Record telemetry while monitoring; --samples selects headless recording')
            [CompletionResult]::new('replay', 'replay', [CompletionResultType]::ParameterValue, 'Replay a local recording; --samples selects JSON output')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export live telemetry or a recording to CSV or SVG')
            [CompletionResult]::new('gpu', 'gpu', [CompletionResultType]::ParameterValue, 'Open the GPU view or select CUDA devices')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Open connection monitoring, test a target, or capture packets')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Explain resource problems, recent trends and useful next steps')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Print the configuration path or write current preferences')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions without reading configuration')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Print the manual page without reading configuration')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update this installer-managed copy of rstop')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;monitor' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;snapshot' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;sample' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;record' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;replay' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;export' {
            [CompletionResult]::new('--input', '--input', [CompletionResultType]::ParameterName, 'input')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;gpu' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('select', 'select', [CompletionResultType]::ParameterValue, 'Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;gpu;select' {
            [CompletionResult]::new('--count', '--count', [CompletionResultType]::ParameterName, 'count')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;gpu;help' {
            [CompletionResult]::new('select', 'select', [CompletionResultType]::ParameterValue, 'Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;gpu;help;select' {
            break
        }
        'rstop;gpu;help;help' {
            break
        }
        'rstop;network' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('connections', 'connections', [CompletionResultType]::ParameterValue, 'List sockets and their owners; samples TCP counters twice for rates')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Check an HTTP(S) URL or TCP host:port, with bounded timeouts')
            [CompletionResult]::new('capture', 'capture', [CompletionResultType]::ParameterValue, 'Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Explain and preview the first 60 headers in a local PCAP')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;network;connections' {
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'AND terms: port:443 process:python state:LISTEN pid:42 !proto:udp')
            [CompletionResult]::new('--sort', '--sort', [CompletionResultType]::ParameterName, 'Choose a network field; missing values always sort last')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--reverse', '--reverse', [CompletionResultType]::ParameterName, 'Reverse the chosen field''s default direction')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;network;check' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;network;capture' {
            [CompletionResult]::new('--interface', '--interface', [CompletionResultType]::ParameterName, 'interface')
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'filter')
            [CompletionResult]::new('--seconds', '--seconds', [CompletionResultType]::ParameterName, 'seconds')
            [CompletionResult]::new('--packets', '--packets', [CompletionResultType]::ParameterName, 'packets')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;network;read' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;network;help' {
            [CompletionResult]::new('connections', 'connections', [CompletionResultType]::ParameterValue, 'List sockets and their owners; samples TCP counters twice for rates')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Check an HTTP(S) URL or TCP host:port, with bounded timeouts')
            [CompletionResult]::new('capture', 'capture', [CompletionResultType]::ParameterValue, 'Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Explain and preview the first 60 headers in a local PCAP')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;network;help;connections' {
            break
        }
        'rstop;network;help;check' {
            break
        }
        'rstop;network;help;capture' {
            break
        }
        'rstop;network;help;read' {
            break
        }
        'rstop;network;help;help' {
            break
        }
        'rstop;doctor' {
            [CompletionResult]::new('--input', '--input', [CompletionResultType]::ParameterName, 'Diagnose an existing recording without inspecting this machine')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Print structured findings and the explanation as JSON')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;config' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('path', 'path', [CompletionResultType]::ParameterValue, 'Print the configuration path')
            [CompletionResult]::new('write', 'write', [CompletionResultType]::ParameterValue, 'Save preferences to the configuration path')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;config;path' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;config;write' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;config;help' {
            [CompletionResult]::new('path', 'path', [CompletionResultType]::ParameterValue, 'Print the configuration path')
            [CompletionResult]::new('write', 'write', [CompletionResultType]::ParameterValue, 'Save preferences to the configuration path')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;config;help;path' {
            break
        }
        'rstop;config;help;write' {
            break
        }
        'rstop;config;help;help' {
            break
        }
        'rstop;completions' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;man' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;update' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Sampling interval in milliseconds (250..=10000)')
            [CompletionResult]::new('--theme', '--theme', [CompletionResultType]::ParameterName, 'Color theme (also adjustable in F2 settings)')
            [CompletionResult]::new('--view', '--view', [CompletionResultType]::ParameterName, 'view')
            [CompletionResult]::new('--gpu-panel', '--gpu-panel', [CompletionResultType]::ParameterName, 'Initial GPU panel focus; all panels remain on one page (history aliases summary)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Alternate configuration file')
            [CompletionResult]::new('--samples', '--samples', [CompletionResultType]::ParameterName, 'Number of samples; emits newline-delimited JSON, or fills SVG graph history')
            [CompletionResult]::new('--export-svg', '--export-svg', [CompletionResultType]::ParameterName, 'Render real collected data to an SVG, without opening a terminal')
            [CompletionResult]::new('--width', '--width', [CompletionResultType]::ParameterName, 'width')
            [CompletionResult]::new('--height', '--height', [CompletionResultType]::ParameterName, 'height')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'record')
            [CompletionResult]::new('--csv', '--csv', [CompletionResultType]::ParameterName, 'csv')
            [CompletionResult]::new('--replay', '--replay', [CompletionResultType]::ParameterName, 'replay')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'layout')
            [CompletionResult]::new('--select-gpu', '--select-gpu', [CompletionResultType]::ParameterName, 'Print CUDA_VISIBLE_DEVICES for N least busy devices meeting the free-memory limit')
            [CompletionResult]::new('--min-free-mib', '--min-free-mib', [CompletionResultType]::ParameterName, 'min-free-mib')
            [CompletionResult]::new('--min-total-mib', '--min-total-mib', [CompletionResultType]::ParameterName, 'Minimum total VRAM for GPU selection')
            [CompletionResult]::new('--max-gpu-utilization', '--max-gpu-utilization', [CompletionResultType]::ParameterName, 'GPU selection ceiling; devices with unknown utilization are excluded')
            [CompletionResult]::new('--max-memory-utilization', '--max-memory-utilization', [CompletionResultType]::ParameterName, 'GPU selection memory-controller utilization ceiling')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Check availability without installing')
            [CompletionResult]::new('--no-gpu', '--no-gpu', [CompletionResultType]::ParameterName, 'Disable GPU driver queries')
            [CompletionResult]::new('--snapshot', '--snapshot', [CompletionResultType]::ParameterName, 'Print a JSON snapshot, then exit (includes process commands)')
            [CompletionResult]::new('--write-config', '--write-config', [CompletionResultType]::ParameterName, 'Write preferences to the configuration file, then exit')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'ascii')
            [CompletionResult]::new('--no-color', '--no-color', [CompletionResultType]::ParameterName, 'no-color')
            [CompletionResult]::new('--threads', '--threads', [CompletionResultType]::ParameterName, 'threads')
            [CompletionResult]::new('--process-network', '--process-network', [CompletionResultType]::ParameterName, 'process-network')
            [CompletionResult]::new('--read-only', '--read-only', [CompletionResultType]::ParameterName, 'read-only')
            [CompletionResult]::new('--full-sampling', '--full-sampling', [CompletionResultType]::ParameterName, 'Preload all pages and keep their background sampling active')
            [CompletionResult]::new('--no-config-exec', '--no-config-exec', [CompletionResultType]::ParameterName, 'Disable configured probes and alert commands')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rstop;help' {
            [CompletionResult]::new('monitor', 'monitor', [CompletionResultType]::ParameterValue, 'Open the interactive system monitor (also the default command)')
            [CompletionResult]::new('snapshot', 'snapshot', [CompletionResultType]::ParameterValue, 'Print one JSON snapshot, including process command lines')
            [CompletionResult]::new('sample', 'sample', [CompletionResultType]::ParameterValue, 'Print newline-delimited JSON samples (ten by default)')
            [CompletionResult]::new('record', 'record', [CompletionResultType]::ParameterValue, 'Record telemetry while monitoring; --samples selects headless recording')
            [CompletionResult]::new('replay', 'replay', [CompletionResultType]::ParameterValue, 'Replay a local recording; --samples selects JSON output')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export live telemetry or a recording to CSV or SVG')
            [CompletionResult]::new('gpu', 'gpu', [CompletionResultType]::ParameterValue, 'Open the GPU view or select CUDA devices')
            [CompletionResult]::new('network', 'network', [CompletionResultType]::ParameterValue, 'Open connection monitoring, test a target, or capture packets')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Explain resource problems, recent trends and useful next steps')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Print the configuration path or write current preferences')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions without reading configuration')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Print the manual page without reading configuration')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update this installer-managed copy of rstop')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rstop;help;monitor' {
            break
        }
        'rstop;help;snapshot' {
            break
        }
        'rstop;help;sample' {
            break
        }
        'rstop;help;record' {
            break
        }
        'rstop;help;replay' {
            break
        }
        'rstop;help;export' {
            break
        }
        'rstop;help;gpu' {
            [CompletionResult]::new('select', 'select', [CompletionResultType]::ParameterValue, 'Print CUDA_VISIBLE_DEVICES for the least busy eligible GPUs')
            break
        }
        'rstop;help;gpu;select' {
            break
        }
        'rstop;help;network' {
            [CompletionResult]::new('connections', 'connections', [CompletionResultType]::ParameterValue, 'List sockets and their owners; samples TCP counters twice for rates')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Check an HTTP(S) URL or TCP host:port, with bounded timeouts')
            [CompletionResult]::new('capture', 'capture', [CompletionResultType]::ParameterValue, 'Save a short private PCAP with tcpdump (Linux/macOS; existing privileges)')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Explain and preview the first 60 headers in a local PCAP')
            break
        }
        'rstop;help;network;connections' {
            break
        }
        'rstop;help;network;check' {
            break
        }
        'rstop;help;network;capture' {
            break
        }
        'rstop;help;network;read' {
            break
        }
        'rstop;help;doctor' {
            break
        }
        'rstop;help;config' {
            [CompletionResult]::new('path', 'path', [CompletionResultType]::ParameterValue, 'Print the configuration path')
            [CompletionResult]::new('write', 'write', [CompletionResultType]::ParameterValue, 'Save preferences to the configuration path')
            break
        }
        'rstop;help;config;path' {
            break
        }
        'rstop;help;config;write' {
            break
        }
        'rstop;help;completions' {
            break
        }
        'rstop;help;man' {
            break
        }
        'rstop;help;update' {
            break
        }
        'rstop;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
