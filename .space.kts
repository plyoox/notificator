job("Build and push Docker") {
    startOn {
        gitPush {
            branchFilter {
                +"refs/heads/main"
            }
        }
    }

    host {
        dockerBuildPush {
            file = "Dockerfile"
            labels["vendor"] = "Plyoox"

            val spaceRepo = "plyoox.registry.jetbrains.space/p/plyoox/plyoox/$spaceRepo"
            // image tags for 'docker push'
            tags {
                +"$spaceRepo:latest"
            }
        }
    }
}