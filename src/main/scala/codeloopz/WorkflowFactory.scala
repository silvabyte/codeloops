package codeloopz

trait WorkflowFactory:
  def runWithArgs(args: Seq[String]): Either[WorkflowError, String]
