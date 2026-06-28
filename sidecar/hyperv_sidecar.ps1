# hyperv_sidecar.ps1
$ErrorActionPreference = 'Stop'

function Send-Response {
    param(
        [Parameter(Mandatory)] [object] $Id,
        [Parameter(Mandatory)] [bool] $Success,
        [Parameter()] [AllowNull()] [object] $Data,
        [Parameter()] [AllowNull()] [object] $Error
    )
    $response = [ordered]@{
        id = $Id
        success = $Success
        data = $Data
        error = $Error
    }
    $json = $response | ConvertTo-Json -Compress -Depth 10
    [Console]::Out.WriteLine($json)
}

while ($true) {
    $line = [Console]::In.ReadLine()
    if ($null -eq $line) { break }

    $request = $line | ConvertFrom-Json
    $id = $request.id
    $command = $request.command

    try {
        $result = Invoke-Expression -Command $command
        # If the command produced a single string (e.g. already JSON from ConvertTo-Json),
        # use it directly. Otherwise serialize the object(s) to JSON.
        if ($result -is [string]) {
            $data = $result
        } elseif ($result -is [array]) {
            $data = $result | ConvertTo-Json -Compress -Depth 10
        } else {
            $data = $result | ConvertTo-Json -Compress -Depth 10
        }
        Send-Response -Id $id -Success $true -Data $data -Error $null
    } catch {
        $errorRecord = @{
            Message = $_.Exception.Message
            Category = $_.CategoryInfo.Category.ToString()
            FullyQualifiedErrorId = $_.FullyQualifiedErrorId
        }
        Send-Response -Id $id -Success $false -Data $null -Error $errorRecord
    }
}
