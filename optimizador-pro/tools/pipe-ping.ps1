# Prueba rápida del servicio: envía {"cmd":"ping"} y {"cmd":"scan"} por el named pipe.
$pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", "optimizador-pro", [System.IO.Pipes.PipeDirection]::InOut)
$pipe.Connect(3000)
$w = New-Object System.IO.StreamWriter($pipe); $w.AutoFlush = $true
$r = New-Object System.IO.StreamReader($pipe)
$w.WriteLine('{"cmd":"ping"}');  $r.ReadLine()
$w.WriteLine('{"cmd":"scan"}');  $r.ReadLine()
$pipe.Dispose()
