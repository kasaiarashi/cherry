// Copyright Krishna Teja Mekala. All Rights Reserved.

#include "Server/CherryLinkServer.h"
#include "CherryLinkModule.h"

#include "Async/Async.h"
#include "Misc/DateTime.h"
#include "Misc/App.h"
#include "Misc/Paths.h"
#include "Misc/EngineVersion.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"
#include "Dom/JsonObject.h"

FCherryLinkServer::FCherryLinkServer()
	: bShouldStop(false)
	, bIsRunning(false)
	, bIsConnected(false)
{
}

FCherryLinkServer::~FCherryLinkServer()
{
	Stop();
}

bool FCherryLinkServer::Start(int32 Port)
{
	if (bIsRunning)
	{
		UE_LOG(LogCherryLink, Warning, TEXT("Server already running"));
		return false;
	}

	if (!CreateListenSocket(Port))
	{
		return false;
	}

	CurrentPort = Port;
	bShouldStop = false;

	Thread = FRunnableThread::Create(this, TEXT("CherryLinkServer"), 0, TPri_Normal);
	if (!Thread)
	{
		UE_LOG(LogCherryLink, Error, TEXT("Failed to create server thread"));
		if (ListenSocket)
		{
			ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)->DestroySocket(ListenSocket);
			ListenSocket = nullptr;
		}
		return false;
	}

	bIsRunning = true;
	return true;
}

void FCherryLinkServer::Stop()
{
	if (!bIsRunning)
	{
		return;
	}

	bShouldStop = true;
	bIsRunning = false;

	// Close sockets first to unblock any blocking operations
	CloseClientConnection();

	if (ListenSocket)
	{
		ListenSocket->Close();
		ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)->DestroySocket(ListenSocket);
		ListenSocket = nullptr;
	}

	// Wait for and clean up thread
	if (Thread)
	{
		Thread->WaitForCompletion();
		delete Thread;
		Thread = nullptr;
	}
}

bool FCherryLinkServer::CreateListenSocket(int32 Port)
{
	ISocketSubsystem* SocketSubsystem = ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM);
	if (!SocketSubsystem)
	{
		UE_LOG(LogCherryLink, Error, TEXT("Failed to get socket subsystem"));
		return false;
	}

	ListenSocket = SocketSubsystem->CreateSocket(NAME_Stream, TEXT("CherryLinkServer"), false);
	if (!ListenSocket)
	{
		UE_LOG(LogCherryLink, Error, TEXT("Failed to create listen socket"));
		return false;
	}

	ListenSocket->SetReuseAddr(true);
	ListenSocket->SetNonBlocking(true);

	TSharedRef<FInternetAddr> Addr = SocketSubsystem->CreateInternetAddr();
	Addr->SetAnyAddress();
	Addr->SetPort(Port);

	if (!ListenSocket->Bind(*Addr))
	{
		UE_LOG(LogCherryLink, Error, TEXT("Failed to bind socket to port %d"), Port);
		SocketSubsystem->DestroySocket(ListenSocket);
		ListenSocket = nullptr;
		return false;
	}

	if (!ListenSocket->Listen(1))
	{
		UE_LOG(LogCherryLink, Error, TEXT("Failed to listen on socket"));
		SocketSubsystem->DestroySocket(ListenSocket);
		ListenSocket = nullptr;
		return false;
	}

	UE_LOG(LogCherryLink, Log, TEXT("Server listening on port %d"), Port);
	return true;
}

bool FCherryLinkServer::Init()
{
	return true;
}

uint32 FCherryLinkServer::Run()
{
	while (!bShouldStop)
	{
		// Accept new connections if no client connected
		if (!ClientSocket)
		{
			AcceptConnection();
		}

		// Handle connected client
		if (ClientSocket && bIsConnected)
		{
			// Send queued outgoing messages
			FString OutMessage;
			while (OutgoingMessages.Dequeue(OutMessage))
			{
				if (!SendMessageInternal(ClientSocket, OutMessage))
				{
					UE_LOG(LogCherryLink, Warning, TEXT("Failed to send message, closing connection"));
					CloseClientConnection();
					break;
				}
			}

			// Receive and process incoming messages
			if (ClientSocket)
			{
				HandleClientData();
			}
		}

		// Small sleep to prevent busy-waiting
		FPlatformProcess::Sleep(0.01f);
	}

	return 0;
}

void FCherryLinkServer::Exit()
{
	CloseClientConnection();
}

void FCherryLinkServer::AcceptConnection()
{
	if (!ListenSocket)
	{
		return;
	}

	bool bHasPendingConnection = false;
	if (ListenSocket->HasPendingConnection(bHasPendingConnection) && bHasPendingConnection)
	{
		TSharedRef<FInternetAddr> RemoteAddr = ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)->CreateInternetAddr();
		ClientSocket = ListenSocket->Accept(*RemoteAddr, TEXT("CherryLinkClient"));

		if (ClientSocket)
		{
			ClientSocket->SetNonBlocking(true);
			ClientSocket->SetNoDelay(true);
			bIsConnected = true;

			UE_LOG(LogCherryLink, Log, TEXT("Client connected from %s"), *RemoteAddr->ToString(true));

			// Fire event on game thread
			AsyncTask(ENamedThreads::GameThread, [this]()
			{
				OnClientConnected.Broadcast();
			});
		}
	}
}

void FCherryLinkServer::HandleClientData()
{
	if (!ClientSocket)
	{
		return;
	}

	// Check connection state
	ESocketConnectionState State = ClientSocket->GetConnectionState();
	if (State != SCS_Connected)
	{
		UE_LOG(LogCherryLink, Log, TEXT("Client disconnected (connection state: %d)"), static_cast<int32>(State));
		CloseClientConnection();
		return;
	}

	// Try to receive message (temporarily ignoring HasPendingData check for debugging)
	FString Message;
	if (ReceiveMessage(ClientSocket, Message))
	{
		UE_LOG(LogCherryLink, Log, TEXT("Received message from client: %s"), *Message.Left(200));
		ProcessMessage(Message);
	}
}

bool FCherryLinkServer::ReceiveMessage(FSocket* Socket, FString& OutMessage)
{
	if (!Socket)
	{
		return false;
	}

	// Read 4-byte length prefix (big-endian)
	uint8 LengthBuffer[4];
	int32 BytesRead = 0;

	// Use None instead of WaitAll for non-blocking sockets
	if (!Socket->Recv(LengthBuffer, 4, BytesRead, ESocketReceiveFlags::None))
	{
		return false;
	}

	if (BytesRead != 4)
	{
		// Not enough data yet, try again later
		return false;
	}

	uint32 MessageLength = (static_cast<uint32>(LengthBuffer[0]) << 24) |
	                       (static_cast<uint32>(LengthBuffer[1]) << 16) |
	                       (static_cast<uint32>(LengthBuffer[2]) << 8) |
	                       static_cast<uint32>(LengthBuffer[3]);

	if (MessageLength == 0 || MessageLength > 10 * 1024 * 1024) // Max 10MB
	{
		UE_LOG(LogCherryLink, Warning, TEXT("Invalid message length: %u"), MessageLength);
		return false;
	}

	// Read message body
	TArray<uint8> MessageBuffer;
	MessageBuffer.SetNumUninitialized(MessageLength + 1);

	int32 TotalRead = 0;
	while (TotalRead < static_cast<int32>(MessageLength))
	{
		int32 Read = 0;
		// Use None instead of WaitAll for non-blocking sockets
		if (!Socket->Recv(MessageBuffer.GetData() + TotalRead, MessageLength - TotalRead, Read, ESocketReceiveFlags::None))
		{
			return false;
		}

		if (Read <= 0)
		{
			// No more data available right now, will try again on next iteration
			return false;
		}

		TotalRead += Read;
	}

	MessageBuffer[MessageLength] = 0; // Null terminate
	OutMessage = UTF8_TO_TCHAR(reinterpret_cast<const char*>(MessageBuffer.GetData()));

	return true;
}

bool FCherryLinkServer::SendMessageInternal(FSocket* Socket, const FString& Message)
{
	if (!Socket)
	{
		return false;
	}

	FScopeLock Lock(&SendLock);

	FTCHARToUTF8 Converter(*Message);
	const uint8* Data = reinterpret_cast<const uint8*>(Converter.Get());
	int32 DataLen = Converter.Length();

	// Write 4-byte length prefix (big-endian)
	uint8 LengthBuffer[4];
	LengthBuffer[0] = static_cast<uint8>((DataLen >> 24) & 0xFF);
	LengthBuffer[1] = static_cast<uint8>((DataLen >> 16) & 0xFF);
	LengthBuffer[2] = static_cast<uint8>((DataLen >> 8) & 0xFF);
	LengthBuffer[3] = static_cast<uint8>(DataLen & 0xFF);

	int32 BytesSent = 0;
	if (!Socket->Send(LengthBuffer, 4, BytesSent) || BytesSent != 4)
	{
		return false;
	}

	// Send message body
	int32 TotalSent = 0;
	while (TotalSent < DataLen)
	{
		int32 Sent = 0;
		if (!Socket->Send(Data + TotalSent, DataLen - TotalSent, Sent))
		{
			return false;
		}

		if (Sent <= 0)
		{
			return false;
		}

		TotalSent += Sent;
	}

	return true;
}

void FCherryLinkServer::SendMessage(const FString& JsonMessage)
{
	OutgoingMessages.Enqueue(JsonMessage);
}

void FCherryLinkServer::SendNotification(const FString& Method, const TSharedPtr<FJsonObject>& Params)
{
	TSharedPtr<FJsonObject> Notification = MakeShared<FJsonObject>();
	Notification->SetStringField(TEXT("jsonrpc"), TEXT("2.0"));
	Notification->SetStringField(TEXT("method"), Method);

	if (Params.IsValid())
	{
		Notification->SetObjectField(TEXT("params"), Params);
	}

	FString JsonString;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&JsonString);
	FJsonSerializer::Serialize(Notification.ToSharedRef(), Writer);

	SendMessage(JsonString);
}

void FCherryLinkServer::SendResponse(const FString& RequestId, const TSharedPtr<FJsonValue>& Result)
{
	TSharedPtr<FJsonObject> Response = MakeShared<FJsonObject>();
	Response->SetStringField(TEXT("jsonrpc"), TEXT("2.0"));
	Response->SetStringField(TEXT("id"), RequestId);

	if (Result.IsValid())
	{
		Response->SetField(TEXT("result"), Result);
	}
	else
	{
		Response->SetObjectField(TEXT("result"), MakeShared<FJsonObject>());
	}

	FString JsonString;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&JsonString);
	FJsonSerializer::Serialize(Response.ToSharedRef(), Writer);

	SendMessage(JsonString);
}

void FCherryLinkServer::SendError(const FString& RequestId, int32 Code, const FString& Message)
{
	TSharedPtr<FJsonObject> Error = MakeShared<FJsonObject>();
	Error->SetNumberField(TEXT("code"), Code);
	Error->SetStringField(TEXT("message"), Message);

	TSharedPtr<FJsonObject> Response = MakeShared<FJsonObject>();
	Response->SetStringField(TEXT("jsonrpc"), TEXT("2.0"));
	Response->SetStringField(TEXT("id"), RequestId);
	Response->SetObjectField(TEXT("error"), Error);

	FString JsonString;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&JsonString);
	FJsonSerializer::Serialize(Response.ToSharedRef(), Writer);

	SendMessage(JsonString);
}

void FCherryLinkServer::ProcessMessage(const FString& JsonMessage)
{
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonMessage);

	if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
	{
		UE_LOG(LogCherryLink, Warning, TEXT("Failed to parse JSON message"));
		return;
	}

	// Check for JSON-RPC version
	FString JsonRpcVersion;
	if (!JsonObject->TryGetStringField(TEXT("jsonrpc"), JsonRpcVersion) || JsonRpcVersion != TEXT("2.0"))
	{
		UE_LOG(LogCherryLink, Warning, TEXT("Invalid JSON-RPC version"));
		return;
	}

	// Get method (required for requests/notifications)
	FString Method;
	if (JsonObject->TryGetStringField(TEXT("method"), Method))
	{
		FString Id;
		JsonObject->TryGetStringField(TEXT("id"), Id);

		const TSharedPtr<FJsonObject>* Params = nullptr;
		JsonObject->TryGetObjectField(TEXT("params"), Params);

		if (!Id.IsEmpty())
		{
			// This is a request
			HandleRequest(Id, Method, Params ? *Params : nullptr);
		}
		else
		{
			// This is a notification - fire event on game thread
			FString MessageCopy = JsonMessage;
			AsyncTask(ENamedThreads::GameThread, [this, MessageCopy]()
			{
				OnMessageReceived.Broadcast(MessageCopy);
			});
		}
	}
}

void FCherryLinkServer::HandleRequest(const FString& Id, const FString& Method, const TSharedPtr<FJsonObject>& Params)
{
	UE_LOG(LogCherryLink, Log, TEXT("Received request: %s (id: %s)"), *Method, *Id);

	// Handle connection/initialize handshake
	if (Method == TEXT("connection/initialize"))
	{
		TSharedPtr<FJsonObject> Result = MakeShared<FJsonObject>();
		Result->SetStringField(TEXT("serverVersion"), TEXT("1.0.0"));
		Result->SetStringField(TEXT("protocolVersion"), TEXT("1.0"));
		Result->SetStringField(TEXT("engineVersion"), FEngineVersion::Current().ToString());
		Result->SetStringField(TEXT("projectName"), FApp::GetProjectName());
		Result->SetStringField(TEXT("projectPath"), FPaths::ProjectDir());

		TArray<TSharedPtr<FJsonValue>> Features;
		Features.Add(MakeShared<FJsonValueString>(TEXT("blueprint")));
		Features.Add(MakeShared<FJsonValueString>(TEXT("logging")));
		Features.Add(MakeShared<FJsonValueString>(TEXT("play")));
		Features.Add(MakeShared<FJsonValueString>(TEXT("build")));
		Result->SetArrayField(TEXT("supportedFeatures"), Features);

		SendResponse(Id, MakeShared<FJsonValueObject>(Result));
		return;
	}

	// Forward request to services via game thread
	// Build JSON string first
	TSharedPtr<FJsonObject> Request = MakeShared<FJsonObject>();
	Request->SetStringField(TEXT("jsonrpc"), TEXT("2.0"));
	Request->SetStringField(TEXT("id"), Id);
	Request->SetStringField(TEXT("method"), Method);
	if (Params.IsValid())
	{
		Request->SetObjectField(TEXT("params"), Params);
	}

	FString JsonString;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&JsonString);
	FJsonSerializer::Serialize(Request.ToSharedRef(), Writer);

	// Now broadcast on game thread
	AsyncTask(ENamedThreads::GameThread, [this, JsonString]()
	{
		OnMessageReceived.Broadcast(JsonString);
	});
}

void FCherryLinkServer::CloseClientConnection()
{
	if (ClientSocket)
	{
		bIsConnected = false;

		ClientSocket->Close();
		ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)->DestroySocket(ClientSocket);
		ClientSocket = nullptr;

		UE_LOG(LogCherryLink, Log, TEXT("Client connection closed"));

		// Fire event on game thread
		AsyncTask(ENamedThreads::GameThread, [this]()
		{
			OnClientDisconnected.Broadcast();
		});
	}
}
