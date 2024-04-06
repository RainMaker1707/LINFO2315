# LINFO2315 - Design of Embedded and real-time systems

Repository for the pseudo Random Number Generator based on environmental data.

## Authors

- Allegaert Hadrien - 07991800
  
## Useful commands

### Launch the container

```sh
docker run -it --rm -v $(pwd)/esp32s3_ffi:/build --device /dev/ttyUSB0  --group-add dialout  --user esp  registry.forge.uclouvain.be/linfo2315/containers/ffi:v5.1
```

### Compilation

```sh
make
```
