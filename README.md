> "botnet" is already cool enough

## Build
#### virus(sample payload)
`cargo zigbuild --bin virus --release --target x86_64-pc-windows-gnu`
#### cook iso for VM 
`xorriso -outdev output.iso -blank as_needed -map target/x86_64-pc-windows-gnu/release/virus.exe /virus.exe`
