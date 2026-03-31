@main def run(workflow: String, args: String*): Unit =
  import codeloopz.*
  import codeloopz.workflows.PullRequestReview

  val result = workflow match
    case "pr-review" => PullRequestReview.runWithArgs(args)
    case other       => Left(WorkflowError(s"Unknown workflow: $other"))

  result match
    case Right(output) => println(output)
    case Left(err)     => System.err.println(s"Error: ${err.message}")
