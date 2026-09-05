param(
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$SourcePng = $(Join-Path (Split-Path -Parent $PSScriptRoot) 'assets\icon.png'),
    [string]$IconIco = $(Join-Path (Split-Path -Parent $PSScriptRoot) 'target\release\fastxcel.ico'),
    [string]$ExePath = $(Join-Path (Split-Path -Parent $PSScriptRoot) 'target\release\fastxcel.exe')
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class NativeResource {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr BeginUpdateResource(string pFileName, bool bDeleteExistingResources);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool UpdateResource(
        IntPtr hUpdate,
        IntPtr lpType,
        IntPtr lpName,
        ushort wLanguage,
        byte[] lpData,
        uint cbData
    );

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool EndUpdateResource(IntPtr hUpdate, bool fDiscard);
}
"@

function New-ResizedPngBytes {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [int]$Size
    )

    $source = [System.Drawing.Image]::FromFile($Path)
    try {
        $bitmap = New-Object System.Drawing.Bitmap $Size, $Size
        try {
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
                $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
                $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                $graphics.Clear([System.Drawing.Color]::Transparent)
                $graphics.DrawImage($source, 0, 0, $Size, $Size)
            } finally {
                $graphics.Dispose()
            }

            $stream = New-Object System.IO.MemoryStream
            try {
                $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
                return $stream.ToArray()
            } finally {
                $stream.Dispose()
            }
        } finally {
            $bitmap.Dispose()
        }
    } finally {
        $source.Dispose()
    }
}

function Write-IconFile {
    param(
        [Parameter(Mandatory = $true)]
        [array]$Entries,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $fileStream = [System.IO.File]::Create($Path)
    try {
        $writer = New-Object System.IO.BinaryWriter($fileStream)
        try {
            $writer.Write([UInt16]0)
            $writer.Write([UInt16]1)
            $writer.Write([UInt16]$Entries.Count)

            $offset = 6 + (16 * $Entries.Count)
            foreach ($entry in $Entries) {
                $size = [int]$entry.Size
                $png = [byte[]]$entry.Bytes
                $dimension = if ($size -ge 256) { [byte]0 } else { [byte]$size }
                $writer.Write($dimension)
                $writer.Write($dimension)
                $writer.Write([byte]0)
                $writer.Write([byte]0)
                $writer.Write([UInt16]1)
                $writer.Write([UInt16]32)
                $writer.Write([UInt32]$png.Length)
                $writer.Write([UInt32]$offset)
                $offset += $png.Length
            }

            foreach ($entry in $Entries) {
                $png = [byte[]]$entry.Bytes
                $fileStream.Write($png, 0, $png.Length)
            }
        } finally {
            $writer.Dispose()
        }
    } finally {
        $fileStream.Dispose()
    }
}

function Update-ExecutableIcon {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ExeFile,
        [Parameter(Mandatory = $true)]
        [array]$Entries
    )

    if (-not (Test-Path $ExeFile)) {
        throw "Executable not found: $ExeFile"
    }

    $resourceHandle = [NativeResource]::BeginUpdateResource($ExeFile, $false)
    if ($resourceHandle -eq [IntPtr]::Zero) {
        throw "BeginUpdateResource failed for $ExeFile"
    }

    $discard = $true
    try {
        $iconType = [IntPtr]14
        $iconResourceType = [IntPtr]3
        $groupStream = New-Object System.IO.MemoryStream
        $groupWriter = New-Object System.IO.BinaryWriter($groupStream)
        try {
            $groupWriter.Write([UInt16]0)
            $groupWriter.Write([UInt16]1)
            $groupWriter.Write([UInt16]$Entries.Count)

            for ($i = 0; $i -lt $Entries.Count; $i++) {
                $entry = $Entries[$i]
                $size = [int]$entry.Size
                $bytes = [byte[]]$entry.Bytes
                $resourceId = [UInt16]($i + 1)

                $dimension = if ($size -ge 256) { [byte]0 } else { [byte]$size }
                $groupWriter.Write($dimension)
                $groupWriter.Write($dimension)
                $groupWriter.Write([byte]0)
                $groupWriter.Write([byte]0)
                $groupWriter.Write([UInt16]1)
                $groupWriter.Write([UInt16]32)
                $groupWriter.Write([UInt32]$bytes.Length)
                $groupWriter.Write([UInt16]$resourceId)

                $resourceName = [IntPtr]::new($resourceId)
                if (-not [NativeResource]::UpdateResource($resourceHandle, $iconResourceType, $resourceName, 0, $bytes, [UInt32]$bytes.Length)) {
                    throw "UpdateResource failed while writing icon $resourceId"
                }
            }

            $groupBytes = $groupStream.ToArray()
            if (-not [NativeResource]::UpdateResource($resourceHandle, $iconType, [IntPtr]::new(1), 0, $groupBytes, [UInt32]$groupBytes.Length)) {
                throw "UpdateResource failed while writing the icon group"
            }
            $discard = $false
        } finally {
            $groupWriter.Dispose()
            $groupStream.Dispose()
        }

    } finally {
        if (-not [NativeResource]::EndUpdateResource($resourceHandle, $discard)) {
            throw "EndUpdateResource failed for $ExeFile"
        }
    }
}

$sizes = 16, 24, 32, 48, 64, 128, 256
$entries = foreach ($size in $sizes) {
    [pscustomobject]@{
        Size = $size
        Bytes = (New-ResizedPngBytes -Path $SourcePng -Size $size)
    }
}

Write-IconFile -Entries $entries -Path $IconIco
Update-ExecutableIcon -ExeFile $ExePath -Entries $entries

$releaseExe = Join-Path $RepoRoot 'release\fastxcel.exe'
Copy-Item -Force $ExePath $releaseExe
Write-Host "Updated icon resources in $ExePath and copied to $releaseExe"
