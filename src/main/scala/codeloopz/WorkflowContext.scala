package codeloopz

case class WorkflowContext(
    workDir: os.Path,
    agent: AgentRunner,
    logger: Logger,
    model: Option[String] = None
)
