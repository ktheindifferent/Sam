FROM rust:latest
MAINTAINER caleb <calebsmithwoolrich@gmail.com>

RUN echo "0.0.000024"

RUN apt update
RUN apt upgrade -y
RUN apt install libx264-dev libssl-dev unzip libavcodec-extra58 python3 pip git git-lfs wget libboost-dev libopencv-dev python3-opencv ffmpeg iputils-ping libasound2-dev libpulse-dev libvorbisidec-dev libvorbis-dev libopus-dev libflac-dev libsoxr-dev alsa-utils libavahi-client-dev avahi-daemon libexpat1-dev -y
RUN pip3 install rivescript pexpect

RUN pip3 install torch torchvision torchaudio --extra-index-url https://download.pytorch.org/whl/cpu

# RUN mkdir -p /app && cd /app \
# && wget -O /app/sam.tar.xz https://osf.opensam.foundation/api/package/download/armv7/sam.tar.xz?oid=lGJUmlu4Bs0Hscp \
# && tar -xf /app/sam.tar.xz && chmod +x /app/sam

RUN mkdir -p /app && git clone https://git.opensam.foundation/sam/sam.git /app \
    && cd /app \
    && cargo build --release  \
    && rm -Rf /app/src  /app/target/release/build /app/target/release/deps /app/target/release/examples/ /app/target/release/incremental/ /app/target/release/native

RUN wget -O /app/libtorch.zip https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-1.13.1%2Bcpu.zip

RUN unzip /app/libtorch.zip -d /app/libtorch



# RUN echo "<<<<debug>>>>"
# RUN echo $LD_LIBRARY_PATH

# Web Port
EXPOSE 8000

# Web Socker Port
EXPOSE 2794

# Snapcast API Port
EXPOSE 1780
EXPOSE 1705
EXPOSE 1704

WORKDIR /app/target/release

ENV LIBTORCH=/app/libtorch/libtorch
ENV LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH

CMD ["./sam"]