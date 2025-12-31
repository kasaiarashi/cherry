// Copyright Krishna Teja Mekala. All Rights Reserved.

#include "Services/BuildService.h"
#include "Server/CherryLinkServer.h"
#include "CherryLinkModule.h"

#include "Dom/JsonObject.h"
#include "Serialization/JsonSerializer.h"

FBuildService::FBuildService(TSharedPtr<FCherryLinkServer> InServer)
	: Server(InServer)
{
}

FBuildService::~FBuildService()
{
	Shutdown();
}

void FBuildService::Initialize()
{
	// Subscribe to server messages
	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (ServerPtr.IsValid())
	{
		OnMessageHandle = ServerPtr->OnMessageReceived.AddRaw(this, &FBuildService::HandleRequest);
	}

	UE_LOG(LogCherryLink, Log, TEXT("BuildService: Initialized"));
}

void FBuildService::Shutdown()
{
	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (ServerPtr.IsValid())
	{
		ServerPtr->OnMessageReceived.Remove(OnMessageHandle);
	}
}

bool FBuildService::TriggerLiveCoding()
{
	UE_LOG(LogCherryLink, Warning, TEXT("BuildService: Live Coding not available"));
	return false;
}

bool FBuildService::CancelLiveCoding()
{
	UE_LOG(LogCherryLink, Warning, TEXT("BuildService: Cancel Live Coding not supported"));
	return false;
}

bool FBuildService::IsLiveCodingEnabled() const
{
	return false;
}

bool FBuildService::IsCompiling() const
{
	return bIsCompiling;
}

void FBuildService::OnPatchComplete()
{
	bIsCompiling = false;
	UE_LOG(LogCherryLink, Log, TEXT("BuildService: Live Coding patch complete"));
	BroadcastBuildStatus(TEXT("Success"));
}

void FBuildService::OnCompilationStarted()
{
	bIsCompiling = true;
	BroadcastBuildStatus(TEXT("Compiling"));
}

void FBuildService::OnCompilationFinished(bool bSuccess)
{
	bIsCompiling = false;
	BroadcastBuildStatus(bSuccess ? TEXT("Success") : TEXT("Failed"));
}

void FBuildService::BroadcastBuildStatus(const FString& Status, float Progress, const FString& CurrentFile)
{
	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (!ServerPtr.IsValid() || !ServerPtr->IsClientConnected())
	{
		return;
	}

	TSharedPtr<FJsonObject> Params = MakeShared<FJsonObject>();
	Params->SetStringField(TEXT("status"), Status);
	Params->SetNumberField(TEXT("progress"), Progress);

	if (!CurrentFile.IsEmpty())
	{
		Params->SetStringField(TEXT("currentFile"), CurrentFile);
	}

	ServerPtr->SendNotification(TEXT("build/status"), Params);
}

void FBuildService::HandleRequest(const FString& JsonMessage)
{
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonMessage);

	if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
	{
		return;
	}

	FString Method;
	if (!JsonObject->TryGetStringField(TEXT("method"), Method))
	{
		return;
	}

	// Only handle build/* methods
	if (!Method.StartsWith(TEXT("build/")))
	{
		return;
	}

	FString Id;
	JsonObject->TryGetStringField(TEXT("id"), Id);

	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (!ServerPtr.IsValid())
	{
		return;
	}

	if (Method == TEXT("build/liveCoding"))
	{
		bool bSuccess = TriggerLiveCoding();

		TSharedPtr<FJsonObject> Result = MakeShared<FJsonObject>();
		Result->SetBoolField(TEXT("success"), bSuccess);
		Result->SetBoolField(TEXT("liveCodingEnabled"), IsLiveCodingEnabled());
		ServerPtr->SendResponse(Id, MakeShared<FJsonValueObject>(Result));
	}
	else if (Method == TEXT("build/getStatus"))
	{
		TSharedPtr<FJsonObject> Result = MakeShared<FJsonObject>();
		Result->SetBoolField(TEXT("isCompiling"), IsCompiling());
		Result->SetBoolField(TEXT("liveCodingEnabled"), IsLiveCodingEnabled());
		ServerPtr->SendResponse(Id, MakeShared<FJsonValueObject>(Result));
	}
}
