FROM chrislusf/seaweedfs:3.85

EXPOSE 8080

CMD ["volume", "-mserver=srv-captain--sam-seaweed-master:9333", "-ip=srv-captain--sam-seaweed-volume", "-ip.bind=0.0.0.0", "-port=8080", "-dir=/data"]
