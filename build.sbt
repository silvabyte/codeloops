val scala3Version = "3.8.2"

lazy val root = project
  .in(file("."))
  .settings(
    name := "codeloopz",
    version := "0.1.0-SNAPSHOT",

    scalaVersion := scala3Version,

    libraryDependencies ++= Seq(
      "org.scalameta" %% "munit" % "1.2.4" % Test,
      "com.lihaoyi" %% "castor" % "0.3.0",
      "com.lihaoyi" %% "upickle" % "4.4.3",
      "com.lihaoyi" %% "os-lib" % "0.11.7",
      "dev.boogieloop" %% "schema" % "0.6.0"
    )
  )
