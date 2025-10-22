import type { WatchNotification } from '../types'

export function parseNotificationValue(value: string): any {
  // The value is a debug-formatted Rust string
  // For now, just return the string, but this could be enhanced
  // to parse specific formats
  return value;
}

export function getNotificationLabel(notification: WatchNotification): string {
  if (notification.variable_name) {
    return notification.variable_name;
  }
  if (notification.value.startsWith('Block(')) {
    // Extract block name from "Block(name)" format
    const match = notification.value.match(/Block\("(.+?)"\)/);
    return match ? `Block: ${match[1]}` : 'Block';
  }
  if (notification.is_stream) {
    return `Stream: ${notification.function_name}`;
  }
  return notification.function_name;
}

export function getNotificationType(notification: WatchNotification): 'variable' | 'block' | 'stream' {
  if (notification.value.startsWith('Block(')) return 'block';
  if (notification.is_stream) return 'stream';
  return 'variable';
}