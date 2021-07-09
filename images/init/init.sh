#!/bin/sh
mkdir content
sed s/{FUNCTION_NAME}/$FN_FUNCTION_NAME/ template/Dockerfile.in > content/Dockerfile
sed s/{FUNCTION_NAME}/$FN_FUNCTION_NAME/ template/Cargo.toml > content/Cargo.toml
sed s/{FUNCTION_NAME}/$FN_FUNCTION_NAME/ template/func.init.yaml > content/func.init.yaml
cp -r template/src content/
tar -C content -cf init.tar .

cat init.tar

rm -rf content
rm init.tar
