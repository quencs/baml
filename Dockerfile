FROM --platform=linux/amd64 ubuntu:22.04


# Install system dependencies and common Rust build tools
RUN apt-get update && \
    apt-get install -y \
        curl \
        git \
        build-essential \
        ca-certificates \
        openssh-server \
        sudo \
        python3 \
        python3-pip \
        nodejs \
        npm


# Install mise (tool version manager)
RUN curl https://mise.jdx.dev/install.sh | bash \
    && ln -s /root/.local/bin/mise /usr/local/bin/mise

ENV PATH="/root/.local/bin:${PATH}"

# Install Rust 1.89
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.89.0
ENV PATH="/root/.cargo/bin:${PATH}"


# Suppress jemalloc info messages
ENV MALLOC_CONF="background_thread:false,stats_print:false"


# Create a working directory
WORKDIR /workspace

# Copy the local source directory into the container
COPY . /workspace


# Expose SSH port
EXPOSE 22

# Set up SSH (optional, for interactive access)
RUN mkdir /var/run/sshd && \
    echo 'root:root' | chpasswd && \
    sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config

# Ensure mise and Rust are activated for all shells
RUN echo 'export PATH="/root/.local/bin:/root/.cargo/bin:$PATH"' >> /root/.bashrc && \
    echo 'eval "$(mise activate bash --shims)"' >> /root/.bashrc && \
    echo 'eval "$(mise activate bash)"' >> /root/.bashrc && \
    echo 'export PATH="/root/.local/bin:/root/.cargo/bin:$PATH"' >> /root/.zshrc && \
    echo 'eval "$(mise activate zsh --shims)"' >> /root/.zshrc && \
    echo 'eval "$(mise activate zsh)"' >> /root/.zshrc

CMD ["/usr/sbin/sshd", "-D"]
