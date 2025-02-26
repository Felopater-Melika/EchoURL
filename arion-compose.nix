{
  project.name = "echourl";

  services.postgres = {
    service.image = "postgres:16";
    service.environment = {
      POSTGRES_USER = "echo";
      POSTGRES_PASSWORD = "1234";
      POSTGRES_DB = "echodb";
    };
    service.volumes = [ "${toString ./.}/postgres-data:/var/lib/postgresql/data" ];
    service.ports = [ "5432:5432" ];
  };
}

