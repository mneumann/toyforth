
class ExecuteMode; end

class CompileMode
  attr_accessor :name
  attr_accessor :seq
end

class CommentMode; end

class VM
  def initialize()
    @words = {
      "DUP" => proc { self.op_dup },
      "DROP" => proc { self.op_drop },
      "SWAP" => proc { self.op_swap },
      "." => proc { self.op_print },
      ":" => proc { self.op_define },
      ";" => proc { self.op_return },
      "(" => proc { self.op_comment_beg },
      ")" => proc { self.op_comment_end },
      "+" => proc { self.op_add },
      "-" => proc { self.op_sub },
      "*" => proc { self.op_mul },
      "/" => proc { self.op_div },
    }

    @data_stack = []
    @mode_stack = []
    @mode = ExecuteMode.new
  end

  def feed(line)
    remainder = line
    while remainder
      token, remainder = remainder.split(" ", 2)
      break if token.nil? or token.empty?

      word = address_of_token(token)

      case @mode
      when ExecuteMode
	word.call
      when CompileMode
	if seq = @mode.seq
	  raise unless @mode.name
	  # we already have a name
	  case token
	  when ";", "(" then
    	    word.call
	  else
	    raise if word.nil?
	    seq.push(word)
	  end
	else
	  # the token is our name
	  @mode.name = token
	  @mode.seq = []
	end

      when CommentMode
	if token == ")"
	  word.call
	else
	  # ignore all other words
	end
      else
	raise
      end
    end
  end

  def address_of_token(token)
    if num = token_to_number(token) then
      proc { op_push_number(num) }
    elsif word = @words[token]
      word
    else
      nil
    end
  end

  def token_to_number(token)
    if token =~ /^\d*$/ then 
      Integer(token)
    else
      nil
    end
  end

  def pop
    @data_stack.pop || raise
  end

  def pop2
    return [pop, pop]
  end

  def push(n)
    @data_stack.push(n)
  end

  def op_dup() @data_stack.push(@data_stack.last || raise) end
  def op_drop() pop end
  def op_swap() a = pop(); b = pop(); push(a); push(b) end
  def op_add() b, a = pop2(); push(a + b) end
  def op_sub() b, a = pop2(); push(a - b) end
  def op_mul() b, a = pop2(); push(a * b) end
  def op_div() b, a = pop2(); push(a / b) end

  def op_push_number(num)
    @data_stack.push(num)
  end

  def op_print()
    print(" ", @data_stack.pop || raise)
  end

  def op_define()
    raise "already in compile mode" if @mode.kind_of? CompileMode
    # enter compile mode
    @mode_stack.push(@mode)
    @mode = CompileMode.new
  end

  def op_return()
    raise "not in compile mode" unless @mode.kind_of? CompileMode
    seq = @mode.seq
    @words[@mode.name] = proc { seq.each {|w| w.call} }
    @mode = @mode_stack.pop || raise
  end

  def op_comment_beg
    raise "already in comment mode" if @mode.kind_of? CommentMode
    @mode_stack.push(@mode)
    @mode = CommentMode.new
  end

  def op_comment_end
    raise "not in comment mode" unless @mode.kind_of? CommentMode
    @mode = @mode_stack.pop || raise
  end

end

vm = VM.new

vm.feed <<EOF
: SUM-OF-SQUARES   ( a b -- c )   DUP *   SWAP   DUP *  +  ;
: SQUARED   ( n -- nsquared )     DUP *  ;
: SUM-OF-SQUARES2   ( a b -- c )  SQUARED  SWAP SQUARED  +  ;
EOF


vm.feed "3 SQUARED ."
vm.feed "3 4 SUM-OF-SQUARES ."
vm.feed "3 4 SUM-OF-SQUARES2 ."

#vm.feed "1 2 . . ( 1 2 3 )"
#vm.feed ": add ( a b - c ) + ;   "
#vm.feed "3 1 2 - + . "
