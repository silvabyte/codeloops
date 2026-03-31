package codeloopz

case class AgentOutput(stdout: String, stderr: String, exitCode: Int)

trait AgentRunner:
  def run(
      prompt: String,
      workDir: os.Path,
      model: Option[String] = None
  ): Either[WorkflowError, AgentOutput]

enum AgentType:
  case ClaudeCode, OpenCode, Cursor

  def buildArgs(
      prompt: String,
      model: Option[String]
  ): Seq[String] = this match
    case ClaudeCode =>
      val base =
        Seq("claude", "--print", "--dangerously-skip-permissions")
      val modelArgs = model.map(m => Seq("--model", m)).getOrElse(Seq.empty)
      base ++ modelArgs ++ Seq("--", prompt)

    case OpenCode =>
      val base = Seq("opencode", "run")
      val modelArgs = model.map(m => Seq("--model", m)).getOrElse(Seq.empty)
      base ++ modelArgs ++ Seq("--prompt", prompt)

    case Cursor =>
      val m = model.getOrElse("opus-4.5-thinking")
      Seq("cursor-agent", "-p", prompt, "--output-format", "text", "--model", m)

  def binaryName: String = this match
    case ClaudeCode => "claude"
    case OpenCode   => "opencode"
    case Cursor     => "cursor-agent"

object AgentType:
  def fromString(name: String): Option[AgentType] = name.toLowerCase match
    case "claude" | "claude-code" | "claudecode" => Some(AgentType.ClaudeCode)
    case "opencode" | "open-code"                => Some(AgentType.OpenCode)
    case "cursor"                                => Some(AgentType.Cursor)
    case _                                       => None

object AgentRunner:
  def apply(agentType: AgentType): AgentRunner = new AgentRunner:
    def run(
        prompt: String,
        workDir: os.Path,
        model: Option[String]
    ): Either[WorkflowError, AgentOutput] =
      val args = agentType.buildArgs(prompt, model)
      try
        val result = os
          .proc(args)
          .call(cwd = workDir, stdin = os.Pipe, check = false)
        Right(AgentOutput(
          stdout = result.out.text(),
          stderr = result.err.text(),
          exitCode = result.exitCode
        ))
      catch case e: Exception => Left(WorkflowError(e.getMessage))

  val claude: AgentRunner = AgentRunner(AgentType.ClaudeCode)
  val opencode: AgentRunner = AgentRunner(AgentType.OpenCode)
  val cursor: AgentRunner = AgentRunner(AgentType.Cursor)

  def fromType(name: String): Option[AgentRunner] =
    AgentType.fromString(name).map(AgentRunner(_))

  def mock(fn: String => String): AgentRunner = new AgentRunner:
    def run(
        prompt: String,
        workDir: os.Path,
        model: Option[String]
    ): Either[WorkflowError, AgentOutput] =
      Right(AgentOutput(stdout = fn(prompt), stderr = "", exitCode = 0))
