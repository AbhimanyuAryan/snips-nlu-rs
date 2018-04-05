#!/bin/sh -ex

 : ${PROJECT_DIR:?"${0##*/} must be invoked as part of an Xcode script phase"}

set -e

VERSION="0.55.0"
SYSTEM=$1

if [ $SYSTEM != ios ] && [ $SYSTEM != macos ]; then
    echo "Given system should be ios or macos"
    exit 1
fi

OUTDIR=$PROJECT_DIR/Dependencies/$SYSTEM

# create final directory
mkdir -p $OUTDIR

if [ -z "$TARGET_BUILD_TYPE" ]
then
    TARGET_BUILD_TYPE=$(echo ${CONFIGURATION} | tr '[:upper:]' '[:lower:]')
fi

if [ -e ../../../target/$TARGET_BUILD_TYPE/libsnips_nlu_ffi.a ] &&
   [ $SYSTEM = macos ]; then
    echo "Using macOS local build"
    cp ../../../target/$TARGET_BUILD_TYPE/libsnips_nlu_ffi.a $OUTDIR
    cp ../../c/libsnips_nlu.h ../../c/module.modulemap $OUTDIR
elif [ $SYSTEM = ios ]; then
    ARCHS_ARRAY=( $ARCHS )
    echo "Attempt to use iOS local build"

    if [ ${#ARCHS_ARRAY[@]} -eq 1 ]; then
        if [ $ARCHS_ARRAY = arm64 ]; then
            ARCHS_ARRAY="aarch64"
        fi
        LOCAL_LIBRARY_PATH=$PROJECT_DIR/../../../target/$ARCHS_ARRAY-apple-ios/$TARGET_BUILD_TYPE/libsnips_nlu_ffi.a
        echo "Targeting only one arch. Trying to copy $LOCAL_LIBRARY_PATH into $OUTDIR"

        if [ -e $LOCAL_LIBRARY_PATH ]; then
            cp $LOCAL_LIBRARY_PATH $OUTDIR
            cp ../../c/libsnips_nlu.h ../../c/module.modulemap $OUTDIR
            exit 0
        else
            echo "Not found. Skipping to remote library."
        fi
    else
        echo "Targeting multiple archs"
        rm $OUTDIR/*
        cp ../../c/libsnips_nlu.h ../../c/module.modulemap $OUTDIR

        should_lipo=true
        for arch in "${ARCHS_ARRAY[@]}"; do
            if [ $arch = arm64 ]; then
                arch="aarch64"
            fi
            LOCAL_LIBRARY_PATH=$PROJECT_DIR/../../../target/$arch-apple-ios/$TARGET_BUILD_TYPE/libsnips_nlu_ffi.a
            echo "Trying to copy $LOCAL_LIBRARY_PATH into $OUTDIR"
            if [ ! -e $LOCAL_LIBRARY_PATH ]; then
                echo "Not found. Skipping to remote library."
                should_lipo=false
                break
            fi
            cp $LOCAL_LIBRARY_PATH $OUTDIR/libsnips_nlu_ffi_$arch.a
        done

        if [ $should_lipo = true ]; then
            echo "Lipo everything into $OUTDIR/libsnips_nlu_ffi.a"
            lipo -create `find $OUTDIR/libsnips_nlu_ffi_*.a` -output $OUTDIR/libsnips_nlu_ffi.a
            exit 0
        fi
    fi
fi

if [ ! -e $PROJECT_DIR/Dependencies/$SYSTEM/libsnips_nlu_ffi.a ] ||
   [ ! -e $PROJECT_DIR/Dependencies/$SYSTEM/libsnips_nlu.h ]; then
    FILENAME=snips-nlu-$SYSTEM.$VERSION.tgz
    echo "Download $FILENAME"
    URL=https://s3.amazonaws.com/snips/snips-nlu-dev/$FILENAME
    cd $PROJECT_DIR/Dependencies/$SYSTEM
    curl -s $URL | tar zx
fi
