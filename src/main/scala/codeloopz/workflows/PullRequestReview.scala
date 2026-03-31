package codeloopz.workflows

import codeloopz.*

//TODO: abstract this to a shared package in prompts or workflow steps
import PullRequestReview.Step.{GetDiff, Synthesize, SignalVsNoise}

class PullRequestReview(
    prUrl:           String,
    agent:           AgentRunner,
    model:           Option[String] = None,
    reviewerPrompts: Seq[Prompt]    = PullRequestReview.defaultReviewers
) extends RunnableWorkflow[String]:

  private def workflow: Workflow[String] = {
    for
      diff <- Workflow.agent(GetDiff, Map("url" -> prUrl))
      filtered <- Workflow.loop(reviewAndFilter(diff, _))
      result <- Workflow.agent(Synthesize, Map("filtered" -> filtered))
    yield result
  }

  private def reviewAndFilter(
      diff:          String,
      priorFeedback: Option[String]
  ): Workflow[String] = {
    val feedbackContext = priorFeedback
      .map(fb => s"\n\nPrior feedback from signal-vs-noise filter to improve upon:\n$fb")
      .getOrElse("")
    for
      reviews <- Workflow.parallel(
        reviewerPrompts.map(p =>
          Workflow.agent(p, Map("diff" -> diff, "prior_feedback" -> feedbackContext))
        )
      )
      filtered <- Workflow.agent(SignalVsNoise, Map("feedback" -> reviews.mkString("\n---\n")))
    yield filtered
  }

  def run(): Either[WorkflowError, String] = {
    workflow.run(
      WorkflowContext(
        workDir = os.pwd,
        agent   = agent,
        logger  = Logger.console,
        model   = model
      )
    )
  }

object PullRequestReview extends WorkflowFactory:
  private val promptsDir: os.Path = os.pwd / "prompts" / "pr-review"

//TODO: abstract this to a shared package in prompts or workflow steps
  enum Step(val prompt: Prompt) extends Prompt.AsPrompt:
    case GetDiff           extends Step(Prompt.Inline("Get the diff for PR: {{url}}"))
    case ReviewSecurity    extends Step(Prompt.FromFile(promptsDir / "reviewer-security.md"))
    case ReviewCorrectness extends Step(Prompt.FromFile(promptsDir / "reviewer-correctness.md"))
    case SignalVsNoise     extends Step(Prompt.FromFile(promptsDir / "signal-vs-noise.md"))
    case Synthesize        extends Step(Prompt.FromFile(promptsDir / "synthesize.md"))

  val defaultReviewers: Seq[Prompt] = Seq(Step.ReviewSecurity, Step.ReviewCorrectness)

  def runWithArgs(args: Seq[String]): Either[WorkflowError, String] = {
    if args.isEmpty then {
      Left(WorkflowError("Usage: pr-review <pr-url> [--agent <name>]"))
    } else {
      val prUrl     = args.head
      val agentName = args
        .sliding(2)
        .collectFirst { case Seq("--agent", name) => name }
        .getOrElse("opencode")
      val agent = AgentRunner
        .fromType(agentName)
        .getOrElse {
          System.err.println(
            s"Unknown agent: $agentName, falling back to claude"
          )
          AgentRunner.claude
        }
      PullRequestReview(prUrl = prUrl, agent = agent).run()
    }
  }
