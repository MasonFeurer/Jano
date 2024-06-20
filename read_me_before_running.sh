# WARNING: This script is not complete
# WARNING: This script is written for Linux Mint and will likely not work on Windows or MacOS or any non-Debian Linux distros.
# WARNING: This script has not been tested on a clean system.

# The current goal is to implement everything in this file under the `jano-cli fix-system` command,
# making it cross-platform in the process.

# This script will:
#  -- install and setup the Android SDK in your home directory
#  -- setup the required enviornment variables
#  -- install Java if not already installed
#  -- install Gradle if not already installed
#  -- install android targets with `rustup`
#  -- install cargo-ndk with `cargo`

# --------Create Android SDK Home--------
# Create the location for the Android SDK to stay.

mkdir "$HOME/android-sdk"

# -------- Install Command Line Tools --------
# The cmdline-tools will be installed to `cmdline-tools/latest`.

cd "$HOME/android-sdk"
mkdir cmdline-tools
cd cmdline-tools

curl "https://dl.google.com/android/repository/commandlinetools-linux-10406996_latest.zip" -o commandlinetools-latest.zip
unzip -q ./commandlinetools-latest.zip
rm ./commandlinetools-latest.zip

# unzip created a folder called `cmdline-tools`, we want to rename that to `latest`
mv cmdline-tools latest


# -------- Finish setup of Android SDK --------
# We will use the `sdkmanager` tool in the just downloaded cmdline-tools to setup the rest of the SDK.

cd cmdline-tools/latest/bin
./sdkmanager "platform-tools"
./sdkmanager "platforms;android-31"
./sdkmanager "build-tools;30.0.3"

# TODO install NDK to ANDROID_HOME/ndk-bundle
# from https://dl.google.com/android/repository/android-ndk-r26b-linux.zip

# Set Enviornment Variables
ENV_FILE="~/.bashrc"

echo "
export ANDROID_HOME=\"$HOME/android-sdk\"
export ANDROID_NDK_ROOT=\"$ANDROID_HOME/ndk-bundle\"

export PATH=\"$ANDROID_HOME/cmdline-tools/latest/bin:$PATH\"
export PATH=\"$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH\"
export PATH=\"/opt/gradle/gradle-8.8/bin:$PATH\"
" | $ENV_FILE
source $ENV_FILE


# Make sure Java is installed
# FIXME: Check if java is installed already

sudo apt install openjdk-17-jdk
sudo apt install openjdk-17-jre

# Install Gradle
# releases are found at: https://gradle.org/releases/
curl -L "https://services.gradle.org/distributions/gradle-8.8-bin.zip" -o gradle-8.8-bin.zip
sudo mkdir /opt/gradle
sudo unzip -d /opt/gradle ./gradle-8.8-bin.zip
rm ./gradle-8.8-bin.zip

# Setup rust for android
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-ndk

# Get Libunwind
# the `libunwind` library is needed by rust.

# cd ~/Downloads
# curl "http://fl.us.mirror.archlinuxarm.org/aarch64/extra/libunwind-1.6.2-2-aarch64.pkg.tar.xz" -o libunwind-1.6.2-2-aarch64.pkg.tar.xz
# tar fx "libunwind-1.6.2-2-aarch64.pkg.tar.xz" usr
# cp "usr/lib/libunwind.so" "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/11.0.5/lib/linux/aarch64"

