package codeloopz

trait RunnableWorkflow[A]:
  def run(): Either[WorkflowError, A]
