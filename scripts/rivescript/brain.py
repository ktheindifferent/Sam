#!/usr/bin/python3

# Python 3 example with context support
import sys
import json
import re
import os
from rivescript import RiveScript
import sentry_sdk

# Initialize Sentry for error tracking
sentry_sdk.init("http://2f7ca9e40bcc42589eb9c01e0a8696ea@sentry.alpha.opensam.foundation/5")

rs = RiveScript()

# Get the script directory and load brain files
script_dir = os.path.dirname(os.path.abspath(__file__))
brain_dir = os.path.join(script_dir, "brain")

# Fallback to /opt/sam if brain directory doesn't exist locally
if not os.path.exists(brain_dir):
    brain_dir = "/opt/sam/scripts/rivescript/brain"

try:
    rs.load_directory(brain_dir)
    rs.sort_replies()
except Exception as e:
    # Log the syntax error to Sentry silently
    sentry_sdk.capture_exception(e)
    # Exit gracefully without printing errors
    sys.exit(1)

# Get user input
user_input = sys.argv[1] if len(sys.argv) > 1 else ""

# Pre-process mathematical expressions to prevent corruption
def preprocess_math_expressions(text):
    """
    Pre-process mathematical expressions before RiveScript substitutions
    to prevent corruption during text processing
    """
    import re
    
    # Handle "whats 2+2" style expressions by adding spaces around operators
    # This prevents RiveScript substitution corruption
    math_patterns = [
        (r'(whats|what\s+is)\s+(\d+)([+\-*/])(\d+)', r'\1 \2 \3 \4'),  # whats 2+2 -> whats 2 + 2
        (r'(\d+)([+\-*/])(\d+)', r'\1 \2 \3'),  # 2+2 -> 2 + 2 (for standalone expressions)
    ]
    
    processed_text = text
    for pattern, replacement in math_patterns:
        processed_text = re.sub(pattern, replacement, processed_text)
    
    return processed_text

# Apply preprocessing
user_input = preprocess_math_expressions(user_input)

# Get context if provided
context_data = {}
if len(sys.argv) > 2:
    try:
        context_data = json.loads(sys.argv[2])
    except:
        context_data = {}

# Set user variables based on context
user_id = "localuser"
if context_data:
    rs.set_uservar(user_id, "current_directory", context_data.get("current_directory", "~"))
    rs.set_uservar(user_id, "recent_commands", json.dumps(context_data.get("recent_commands", [])))
    rs.set_uservar(user_id, "favorite_directories", json.dumps(context_data.get("favorite_directories", [])))
    rs.set_uservar(user_id, "running_services", json.dumps(context_data.get("running_services", [])))
    rs.set_uservar(user_id, "conversation_context", context_data.get("conversation_context", ""))

# Get the reply from RiveScript
reply = rs.reply(user_id, user_input)

# Enhanced response processing for command embedding
def enhance_reply_with_commands(reply_text, user_input):
    """
    Enhance RiveScript replies by embedding appropriate commands
    based on the user's request and context
    """
    
    # Comprehensive command patterns based on help.rs
    command_patterns = {
        # Basic system commands
        r"(help|what.*can.*you.*do|commands|options)": "::::: help :::::",
        r"(start.*http|start.*web|web.*server)": "::::: http start :::::",
        r"(stop.*http|stop.*web)": "::::: http stop :::::",
        r"(system.*status|status|how.*are.*you)": "::::: status :::::",
        r"(services|what.*services|running.*services)": "::::: services :::::",
        r"(version|what.*version)": "::::: version :::::",
        r"(clear.*screen|clear)": "::::: clear :::::",
        
        # File operations
        r"(list.*files|show.*files|what.*files)": "::::: ls :::::",
        r"(current.*directory|where.*am.*i|pwd)": "::::: pwd :::::",
        r"(go.*to.*home|change.*to.*home)": "::::: cd ~ :::::",
        r"(go.*to.*downloads|change.*to.*downloads)": "::::: cd ~/Downloads :::::",
        r"(go.*to.*desktop|change.*to.*desktop)": "::::: cd ~/Desktop :::::",
        r"(go.*to.*documents|change.*to.*documents)": "::::: cd ~/Documents :::::",
        r"(create.*dir|make.*dir|mkdir)": "::::: mkdir :::::",
        r"(remove.*dir|delete.*dir|rmdir)": "::::: rmdir :::::",
        r"(copy.*file|copy)": "::::: cp :::::",
        r"(move.*file|rename|mv)": "::::: mv :::::",
        r"(delete.*file|remove.*file|rm)": "::::: rm :::::",
        r"(show.*file.*content|read.*file|cat)": "::::: cat :::::",
        r"(view.*file|less)": "::::: less :::::",
        r"(edit.*file|nano)": "::::: nano :::::",
        r"(create.*file|touch)": "::::: touch :::::",
        r"(first.*lines|head)": "::::: head :::::",
        r"(last.*lines|tail)": "::::: tail :::::",
        r"(find.*file|search.*file)": "::::: find . -name :::::",
        r"(change.*permission|chmod)": "::::: chmod :::::",
        r"(change.*owner|chown)": "::::: chown :::::",
        r"(search.*text|grep)": "::::: grep :::::",
        r"(sort.*file|sort)": "::::: sort :::::",
        r"(count.*lines|count.*words|wc)": "::::: wc :::::",
        r"(archive|tar)": "::::: tar :::::",
        r"(compress|gzip)": "::::: gzip :::::",
        r"(decompress|gunzip)": "::::: gunzip :::::",
        
        # Text-to-speech
        r"(say|speak|tts)": "::::: tts :::::",
        
        # AI/LLM commands
        r"(install.*llama|setup.*llama)": "::::: llama install :::::",
        r"(llama|ask.*llama)": "::::: llama :::::",
        r"(llama2|ask.*llama2)": "::::: llama2 :::::",
        r"(tiny.*llama|llama.*tiny)": "::::: llama2-tiny :::::",
        
        # LIFX light control
        r"(start.*lifx|turn.*on.*lights|lights.*on)": "::::: lifx start :::::",
        r"(stop.*lifx|turn.*off.*lights|lights.*off)": "::::: lifx stop :::::",
        r"(lifx.*status|light.*status)": "::::: lifx status :::::",
        
        # Web crawler
        r"(start.*crawler|crawler.*start)": "::::: crawler start :::::",
        r"(stop.*crawler|crawler.*stop)": "::::: crawler stop :::::",
        r"(crawler.*status)": "::::: crawler status :::::",
        r"(search.*pages|crawl.*search)": "::::: crawl search :::::",
        
        # Redis
        r"(install.*redis|setup.*redis)": "::::: redis install :::::",
        r"(start.*redis|redis.*start)": "::::: redis start :::::",
        r"(stop.*redis|redis.*stop)": "::::: redis stop :::::",
        r"(redis.*status|check.*redis)": "::::: redis status :::::",
        
        # Docker
        r"(start.*docker|docker.*start)": "::::: docker start :::::",
        r"(stop.*docker|docker.*stop)": "::::: docker stop :::::",
        
        # System monitoring
        r"(disk.*space|storage|df)": "::::: df -h :::::",
        r"(memory.*usage|ram|top)": "::::: top :::::",
        r"(running.*processes|processes|ps)": "::::: ps aux :::::",
        
        # Common cleanup operations
        r"(clear.*downloads.*folder|clean.*downloads|empty.*downloads)": "::::: rm -rf ~/Downloads/* :::::",
        r"(clear.*desktop|clean.*desktop)": "::::: rm -rf ~/Desktop/* :::::",
        r"(clear.*trash|empty.*trash)": "::::: rm -rf ~/.Trash/* :::::",
        
        # Debug operations
        r"(debug.*level|set.*debug)": "::::: debug :::::",
        r"(show.*errors|errors)": "::::: errors :::::",
    }
    
    # Check if the user input matches any command patterns
    user_lower = user_input.lower()
    embedded_command = None
    
    for pattern, command in command_patterns.items():
        if re.search(pattern, user_lower):
            embedded_command = command
            break
    
    # If we found a command to embed, add it to the reply
    if embedded_command:
        # If the reply doesn't already contain a command
        if ":::::" not in reply_text:
            # Add the command to the reply in a natural way
            if "I'll" in reply_text or "I will" in reply_text:
                # Insert command after the statement
                reply_text = reply_text.replace(".", f". {embedded_command}")
            else:
                # Add command at the end
                reply_text = f"{reply_text} {embedded_command}"
    
    return reply_text

# Process the reply for command embedding
enhanced_reply = enhance_reply_with_commands(reply, user_input)

print(enhanced_reply)

# vim:expandtab
