export interface Message {
  id: string;
  content: string;
  timestamp: string;
  isAgent: boolean;
}

interface MessageBubbleProps {
  message: Message;
}

export function MessageBubble({ message }: MessageBubbleProps) {
  if (message.isAgent) {
    return (
      <div className="flex justify-end">
        <div className="max-w-[70%] bg-gradient-to-r from-blue-600 to-purple-600 rounded-2xl rounded-tr-sm p-4">
          <p className="text-white">{message.content}</p>
          <span className="text-xs text-blue-100 mt-2 block">{message.timestamp}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex justify-start">
      <div className="max-w-[70%] bg-gray-100 rounded-2xl rounded-tl-sm p-4">
        <p className="text-gray-800">{message.content}</p>
        <span className="text-xs text-gray-500 mt-2 block">{message.timestamp}</span>
      </div>
    </div>
  );
}
