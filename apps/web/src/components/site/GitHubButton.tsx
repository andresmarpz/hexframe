import { githubUrl } from "../../content/copy";
import { Button, type ButtonSize } from "../kit/Button";

interface Props {
  size?: ButtonSize;
  className?: string;
}

export function GitHubButton({ size = "xxl", className }: Props) {
  return (
    <Button href={githubUrl} variant="outline" size={size} className={className}>
      View on GitHub
    </Button>
  );
}
