interface ActiveToolCallBannerProps {
  toolName: string;
  isActive: boolean;
}

export function ActiveToolCallBanner({ toolName, isActive }: ActiveToolCallBannerProps) {
  if (!isActive) return null;

  return (
    <div className="px-6 py-3 bg-amber-50 border-t border-amber-100">
      <div className="flex items-center gap-2">
        <div className="animate-pulse w-2 h-2 bg-amber-500 rounded-full"></div>
        <span className="text-sm text-amber-800 font-medium">
          Agent is {toolName}...
        </span>
      </div>
    </div>
  );
}
