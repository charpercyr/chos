#!/bin/zsh

ACPICA_VERSION=20211217
ACPICA_DIR=acpica-unix-$ACPICA_VERSION

ROOT=$(realpath $(dirname $0))

pushd $ROOT

# Download & Build ACPICA IASL
mkdir -p externals
pushd externals
if [ ! -f acpica-unix-$ACPICA_VERSION.tar.gz ]; then
    wget https://acpica.org/sites/acpica/files/$ACPICA_DIR.tar.gz -O $ACPICA_DIR.tar.gz
fi
tar xf $ACPICA_DIR.tar.gz
# Build acpica/iasl
make -C $ACPICA_DIR iasl
PATH="$ROOT/externals/$ACPICA_DIR/generate/unix/bin/:$PATH"
popd

pushd edk2
echo 'ACTIVE_PLATFORM = OvmfPkg/OvmfPkgX64.dsc' > Conf/target.txt
echo 'TARGET_ARCH = X64' >> Conf/target.txt

popd

popd
