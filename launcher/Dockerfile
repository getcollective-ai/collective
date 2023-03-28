FROM ubuntu

# Update and install required packages: Update the package list and install the required packages, including the SSH server and any other required packages.
RUN apt-get update && \
    apt-get install -y openssh-server sudo && \
    apt-get clean

# Install ZSH
RUN apt-get update && apt-get install -y zsh

# Install git
RUN apt-get install -y zsh git

# Install curl
RUN apt-get install -y curl

# Rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Set PATH
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustc --version && \
    cargo --version

# Set zsh as default shell
RUN chsh -s /bin/zsh

# Configure SSH server: Create the SSH directory and set the appropriate permissions:
RUN mkdir -p /var/run/sshd

# Expose SSH port: Add the following line to the Dockerfile to expose the SSH port (22 by default) to the host system:
EXPOSE 22

RUN echo 'PS1="$ "' >> ~/.zshrc
# oh-my-zsh
#RUN sh -c "$(wget -O- https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"

# Set the container entrypoint: Set the entrypoint command to start the SSH server when the container is run:
#CMD ["/usr/sbin/sshd", "-D"]
CMD "zsh"
