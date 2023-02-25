docker build . -t plyoox/notificator
docker tag plyoox/notificator plyoox.registry.jetbrains.space/p/plyoox/plyoox/notificator
docker push plyoox.registry.jetbrains.space/p/plyoox/plyoox/notificator