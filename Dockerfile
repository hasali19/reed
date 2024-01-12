FROM alpine:3.17
ARG TARGETPLATFORM
WORKDIR app
COPY artifacts/$TARGETPLATFORM/reed /usr/local/bin/reed
RUN chmod +x /usr/local/bin/reed
CMD ["/usr/local/bin/reed"]
