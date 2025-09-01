#!/bin/bash

# Claude Code SessionEnd Hook - Save conversation transcript
# Receives session data via stdin as JSON

# Read JSON input from stdin
input=$(cat)

# Parse session data
session_id=$(echo "$input" | jq -r '.session_id')
transcript_path=$(echo "$input" | jq -r '.transcript_path')
reason=$(echo "$input" | jq -r '.reason')
cwd=$(echo "$input" | jq -r '.cwd')

# Create history directory
history_dir="$cwd/.claude/history"
mkdir -p "$history_dir"

# Output file
output_file="$history_dir/session-$session_id.md"

echo "💾 Saving session $session_id transcript..."
echo "📁 Source: $transcript_path"
echo "💿 Output: $output_file"
echo "🚪 Reason: $reason"

# Function to convert JSONL message to markdown
format_message() {
    local json_line="$1"
    
    # Extract fields using jq
    local type=$(echo "$json_line" | jq -r '.type // empty')
    local role=$(echo "$json_line" | jq -r '.message.role // empty')
    local timestamp=$(echo "$json_line" | jq -r '.timestamp // empty')
    
    # Skip non-user/assistant messages
    if [[ "$type" != "user" && "$type" != "assistant" ]]; then
        return
    fi
    
    # Extract text content from content array (with error handling)
    local text_content=$(echo "$json_line" | jq -r '
        if (.message.content | type) == "array" then
            [.message.content[] | select(.type == "text") | .text] | join("\n")
        else
            empty
        end
    ' 2>/dev/null)
    
    # Skip if no text content
    if [[ -z "$text_content" || "$text_content" == "null" ]]; then
        return
    fi
    
    # Format timestamp for readability
    local formatted_time
    if [[ -n "$timestamp" && "$timestamp" != "null" ]]; then
        formatted_time=$(date -j -f "%Y-%m-%dT%H:%M:%S" "$(echo $timestamp | cut -d'.' -f1)" "+%Y-%m-%d %H:%M:%S" 2>/dev/null || echo "$timestamp")
    else
        formatted_time="Unknown"
    fi
    
    # Create markdown entry with proper role formatting
    echo ""
    if [[ "$role" == "user" ]]; then
        echo "## 👤 Human - $formatted_time"
    elif [[ "$role" == "assistant" ]]; then
        echo "## 🤖 Assistant - $formatted_time"
    else
        echo "## $role - $formatted_time"
    fi
    echo ""
    echo "$text_content"
    echo ""
}

# Check if transcript file exists
if [[ ! -f "$transcript_path" ]]; then
    echo "❌ Error: Transcript file not found: $transcript_path"
    exit 1
fi

# Start creating the markdown file
{
    echo "# Claude Session Transcript"
    echo ""
    echo "**Session ID:** \`$session_id\`"
    echo "**End Reason:** $reason"
    echo "**Saved:** $(date)"
    echo "**Working Directory:** \`$cwd\`"
    echo ""
    
    # Count messages for summary
    user_count=$(grep '"type":"user"' "$transcript_path" | grep -c '"role":"user"')
    assistant_count=$(grep '"type":"assistant"' "$transcript_path" | grep -c '"role":"assistant"')
    
    echo "**Summary:** $user_count user messages, $assistant_count assistant responses"
    echo ""
    echo "---"
    
    # Process each line of the JSONL file
    while IFS= read -r line; do
        # Skip empty lines and summary lines
        if [[ -n "$line" && ! "$line" =~ '"type":"summary"' ]]; then
            formatted=$(format_message "$line")
            if [[ -n "$formatted" ]]; then
                echo "$formatted"
            fi
        fi
    done < "$transcript_path"
    
} > "$output_file"

echo "✅ Session transcript saved successfully!"
echo "📊 Messages: $user_count user, $assistant_count assistant"
echo "📄 File: $output_file"