searchState.loadedDescShard("gear_core_processor", 0, "Gear message processor.\nAllocation error\n<code>Ext</code>’s memory management (calls to allocate and free) …\nCharge error\nCharge error\nCharge error\nThe instance returned by <code>precharge_for_code</code>. Existence of …\nThe instance returned by <code>precharge_for_instrumentation</code>. …\nBasic error\nBasic error\nStructure providing externalities for running host …\nFallible API error.\nAn error occurs in attempt to call forbidden syscall.\nChecked parameters for message execution across processing …\nProcessor context.\nTrait to which ext must have to work in processor wasm …\nPossible variants of the <code>DispatchResult</code> if the latter …\nSystem reservation context.\nInfallible API error.\nAllocations context.\nBlock info.\nCommon structures for processing.\nConfigurations.\nProcessor context.\nExecution externalities costs.\nActual gas counter type within wasm module’s global.\nReservation created in current execution.\nExistential deposit.\nFunctions forbidden to be called.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nExtracts reservation context from dispatch.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGas allowance counter.\nGas counter.\nGas multiplier.\nReserved gas counter.\nHandle some journal records passing them to the journal …\nChecks if there are any reservations.\nInformational functions for core-processor and executor.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConvert externalities into info.\nProtect and save storage keys for pages which has no data\nLazy pages program post execution actions\nReturns lazy pages status\nMailbox threshold.\nReturns memory infix.\nMessage context.\nCreate new\nPerformance multiplier.\nPrecharge for allocations obtaining from storage.\nCharge a message for the program binary code beforehand.\nCharge a message for fetching the actual length of the …\nCharge a message for instrumentation of the binary code …\nCharge a message for program memory and module …\nCharge a message for program data beforehand.\nReservation from <code>ContextStore</code>.\nProcess program &amp; dispatch for it and return journal for …\nHelper function for journal creation in trap/error case.\nHelper function for journal creation in message no …\nHelper function for journal creation in case of …\nHelper function for journal creation in success case\nMap of code hashes to program ids of future programs, …\nReturns program id.\nCurrent program id\nOutput from Randomness.\nReserve for parameter of scheduling.\nSystem reservation.\nValue counter.\nActor.\nActor execution error.\nReason of execution error\nDispatch outcome of the specific message.\nResult of the specific dispatch.\nKind of the dispatch result.\nBackend error\nEnvironment error\nExecutable actor data.\nExecution error.\nExit dispatch.\nMessage was a exit.\nExit the program.\nAn error occurs in attempt to call forbidden syscall.\nGas allowance exceed.\nSome gas was burned.\nAn error occurs in attempt to charge more gas than …\nMessage was an initialization failure.\nMessage was an initialization success.\nError during <code>into_ext_info()</code> call\nJournal handler.\nJournal record for the state update.\nIncorrect memory parameters\nMessage was handled and no longer exists.\nMessage was successfully dispatched.\nMessage was a trap.\nMessage was processed, but not executed\nNot enough gas to perform an operation during precharge.\nThe error occurs when a program tries to allocate more …\nCreate deposit for future reply.\nReserve gas.\nMessage was generated.\nSend signal.\nSend value\nStop processing queue.\nStore programs requested by user to be initialized later\nSuccessful dispatch\nMessage was a success.\nSystem execution error\nDo system reservation.\nDo system unreservation in case it is created but not used.\nTrap dispatch.\nTrap explanation\nTermination reason\nUnreserve gas.\nMessage is not supported now\nUpdate allocations set note. And also removes data for …\nUpdate gas reservation map in program.\nUpdate page.\nWait dispatch.\nPut this dispatch in the wait list.\nWake particular message.\nNew allocations set for program if it has been changed.\nSet of wasm pages, which are allocated by the program.\nConvert self into <code>gear_core_errors::SimpleExecutionError</code>.\nList of messages that should be woken.\nProgram value balance.\nExported functions by the program code.\nId of the program code.\nContext store after execution.\nDestination program.\nOriginal dispatch.\nExecutable actor data\nProcess exit dispatch.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGas amount after execution.\nGas amount of the execution.\nProcess gas burned.\nGas reservation map.\nGas amount programs reserved.\nList of generated messages.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nKind of the dispatch.\nThe infix of memory pages in a storage.\nProcess message consumed.\nProcess message dispatch.\nReturn dispatch message id.\nReturn dispatch source program id.\nReturn dispatch message value.\nPage updates.\nNew programs to be created with additional data …\nReturn program id.\nProgram id of actor which was executed.\nError text.\nCreate deposit for future reply.\nList of reply deposits to be provided.\nWhether this execution sent out a reply.\nReserve gas.\nProcess send dispatch.\nSend system signal.\nSend value.\nCount of static memory pages.\nStop processing queue.\nStore new programs in storage.\nCreate partially initialized instance with the kind …\nSystem reservation context.\nDo system reservation.\nDo system unreservation.\nUnreserve gas.\nProcess JournalNote::UpdateAllocations.\nUpdate gas reservations.\nProcess page update.\nProcess send message.\nProcess send message.\nSource of the init message. Funds inheritor.\nId of the program that was successfully exited.\nId of the program that was successfully initialized.\nProgram that was failed initializing.\nProgram that was failed.\nReason of the fail.\nReason of the fail.\nNew allocations set for the program.\nAmount of gas burned.\nAmount of reserved gas.\nAmount of reserved gas.\nAmount of gas for reply.\nMessage that should be woken.\nCollection of program candidate ids and their init message …\nSimple signal error.\nCode hash used to create new programs with ids in …\nNew data of the page.\nAmount of blocks to wait before sending.\nAmount of blocks to wait before waking.\nProgram ID which signal will be sent to.\nNew message with entry point that was generated.\nStored dispatch to be inserted into Waitlist.\nPushes StoredDispatch back to the top of the queue.\nExpected duration of holding.\nHow many blocks reservation will live.\nBlock number until reservation will live.\nValue sender\nFuture reply id to be sponsored.\nDecreases gas allowance by that amount, burned for …\nId of the program called <code>exit</code>.\nMessage id of dispatched message.\nMessage id in which gas was burned.\nMessage id of the message that generated this message.\nMessage which has initiated wake.\nMessage from which gas is reserved.\nMessage ID which system reservation will be made from.\nMessage ID which system reservation was made from.\nMessage ID which system reservation was made from.\nMessage id of the message that generated this message.\nOutcome of the processing.\nNumber of the page.\nProgram which has initiated wake.\nProgram that owns the page.\nProgram id.\nCurrent program id.\nProgram which contains reservation.\nProgram which contains reservation.\nProgram whose map will be updated.\nWhether use supply from reservation or current message.\nReservation ID\nReservation ID\nMap with reservations.\nSource of the dispatched message.\nValue beneficiary,\nValue amount\nAddress where all remaining value of the program should be …\nIf this message is waiting for its reincarnation.\nStable parameters for the whole block across processing …\nContextual block information.\nExecution externalities costs.\nModule instantiation costs.\nCosts for message processing\nHolding in storages rent costs.\nAll available syscalls.\nNumber of max pages number to use it in tests.\nReturns iterator of all syscalls.\nBlock info.\nWASM module code section instantiation per byte cost.\nProgram processing costs.\nWASM module data section instantiation per byte cost.\nHolding message in dispatch stash cost per block.\nWASM module element section instantiation per byte cost.\nExistential deposit.\nExecution externalities costs.\nForbidden functions.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a mutable reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGet a reference to the inner from the outer.\nGas multiplier.\nWASM module global section instantiation per byte cost.\nHeight.\nModule instantiation costs.\nReturns iterator of all syscall names (actually supported …\nReturns map of all syscall string values to syscall names.\nCode instrumentation cost.\nCode instrumentation per byte cost.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nChecks whether the syscall is fallible.\nLazy pages costs.\nLoad program allocations cost per interval.\nMailbox threshold.\nMax allowed page numbers for wasm program.\nAmount of reservations can exist for 1 program.\nMemory grow cost.\nMemory grow per page cost.\nOutgoing bytes limit.\nOutgoing limit.\nPerformance multiplier.\nStorage read cost.\nStorage read per byte cost.\nRent costs.\nHolding reservation cost per block.\nReserve for parameter of scheduling.\nChecks whether the syscall returns error either by writing …\nReturns signature for syscall by name.\nSyscalls costs.\nWASM module table section instantiation per byte cost.\nTimestamp.\nReturns name of the syscall.\nWASM module type section instantiation per byte cost.\nHolding message in waitlist cost per block.\nStorage write cost.\n!!! FOR TESTING / INFORMATIONAL USAGE ONLY")