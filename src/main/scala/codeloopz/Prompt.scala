package codeloopz

enum Prompt:
  case Inline(text: String)
  case FromFile(path: os.Path)

  def resolve: Either[WorkflowError, String] = this match
    case Inline(text) => Right(text)
    case FromFile(path) =>
      try Right(os.read(path))
      catch case e: Exception => Left(WorkflowError(e.getMessage))

  def render(inputs: Map[String, String] = Map.empty): Either[WorkflowError, String] =
    resolve.map { text =>
      inputs.foldLeft(text) { (t, kv) =>
        t.replace(s"{{${kv._1}}}", kv._2)
      }
    }

object Prompt:
  given Conversion[String, Prompt] = Prompt.Inline(_)

  trait AsPrompt:
    def prompt: Prompt

  given [A <: AsPrompt]: Conversion[A, Prompt] = _.prompt
