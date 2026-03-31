package codeloopz

trait Logger:
  def info(msg: String): Unit
  def error(msg: String): Unit

object Logger:
  val console: Logger = new Logger:
    def info(msg: String): Unit = println(s"[INFO] $msg")
    def error(msg: String): Unit = System.err.println(s"[ERROR] $msg")

  val silent: Logger = new Logger:
    def info(msg: String): Unit = ()
    def error(msg: String): Unit = ()
