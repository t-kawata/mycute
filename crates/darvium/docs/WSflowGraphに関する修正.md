# WorkflowGraph を以下のように改訂するべき

- Darvium 側の責務  
  - `WorkflowGraph`（DAG）を正本として解釈し、  
    - ready frontier を計算し、  
    - 並列性・条件分岐・SubWorkflow 展開を含む「いつ・どの node を実行するか」を制御する。 
  - `WorkflowNode::AgentStep` を実行する時だけ、その内部で OpenFang に対して  
    - 単一 Agent 呼び出し（Agent）  
    - あるいは OpenFang 側の Workflow（もしサポートするなら）  
    への request/response を行い、その結果を変数に書き戻して「その node が完了した」とみなす。 

- OpenFang 側の責務  
  - Darvium から渡された「この Agent / この Workflow を、この入力で実行してくれ」という 1 回分の要求を処理する **primitive 実行エンジン** に徹する。 
  - グラフ全体の実行順序や frontier 計算は持たず、Darvium から見れば「AgentStep 実行の中の黒箱 API」である。

この前提に立つと:

- `WorkflowNode::AgentStep` … 「OpenFang に投げる 1 回分の仕事」を表す Layer 2 ノード。 
- `WorkflowGraph` … それらの AgentStep / SubWorkflow を DAG として組んだ、Darvium の IR。 
- Darvium executor … `WorkflowGraph` をもとに ready frontier を更新し、node ごとに OpenFang を呼び出す orchestration コア。

という三層がきれいに分離できます。compile_to_stepsは必要ありません。
