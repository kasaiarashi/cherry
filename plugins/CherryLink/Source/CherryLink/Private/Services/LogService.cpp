// Copyright Krishna Teja Mekala. All Rights Reserved.

#include "Services/LogService.h"
#include "Server/CherryLinkServer.h"
#include "CherryLinkModule.h"

#include "Misc/DateTime.h"
#include "Misc/OutputDeviceRedirector.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"

FLogService::FLogService(TSharedPtr<FCherryLinkServer> InServer)
	: Server(InServer)
{
}

FLogService::~FLogService()
{
	Shutdown();
}

void FLogService::Initialize()
{
	if (GLog)
	{
		GLog->AddOutputDevice(this);
		UE_LOG(LogCherryLink, Log, TEXT("LogService: Registered as output device"));
	}

	// Subscribe to server messages
	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (ServerPtr.IsValid())
	{
		OnMessageHandle = ServerPtr->OnMessageReceived.AddRaw(this, &FLogService::HandleRequest);
		UE_LOG(LogCherryLink, Log, TEXT("LogService: Subscribed to server messages"));
	}
}

void FLogService::Shutdown()
{
	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (ServerPtr.IsValid())
	{
		ServerPtr->OnMessageReceived.Remove(OnMessageHandle);
	}

	if (GLog)
	{
		GLog->RemoveOutputDevice(this);
	}
}

void FLogService::SetCategoryFilter(const TArray<FName>& Categories)
{
	FScopeLock Lock(&LogLock);
	FilteredCategories.Empty();
	for (const FName& Category : Categories)
	{
		FilteredCategories.Add(Category);
	}
	bFilterByCategory = FilteredCategories.Num() > 0;
}

void FLogService::SetMinVerbosity(ELogVerbosity::Type Verbosity)
{
	MinVerbosity = Verbosity;
}

void FLogService::ClearFilters()
{
	FScopeLock Lock(&LogLock);
	FilteredCategories.Empty();
	bFilterByCategory = false;
	MinVerbosity = ELogVerbosity::All;
}

void FLogService::Serialize(const TCHAR* Message, ELogVerbosity::Type Verbosity, const FName& Category)
{
	SendLogMessage(Message, Verbosity, Category);
}

void FLogService::Serialize(const TCHAR* Message, ELogVerbosity::Type Verbosity, const FName& Category, const double Time)
{
	SendLogMessage(Message, Verbosity, Category);
}

bool FLogService::ShouldLogCategory(const FName& Category) const
{
	if (!bFilterByCategory)
	{
		return true;
	}
	return FilteredCategories.Contains(Category);
}

FString FLogService::VerbosityToString(ELogVerbosity::Type Verbosity) const
{
	switch (Verbosity)
	{
		case ELogVerbosity::Fatal:       return TEXT("Fatal");
		case ELogVerbosity::Error:       return TEXT("Error");
		case ELogVerbosity::Warning:     return TEXT("Warning");
		case ELogVerbosity::Display:     return TEXT("Display");
		case ELogVerbosity::Log:         return TEXT("Log");
		case ELogVerbosity::Verbose:     return TEXT("Verbose");
		case ELogVerbosity::VeryVerbose: return TEXT("VeryVerbose");
		default:                         return TEXT("Unknown");
	}
}

void FLogService::SendLogMessage(const TCHAR* Message, ELogVerbosity::Type Verbosity, const FName& Category)
{
	if (!bIsEnabled)
	{
		return;
	}

	// CRITICAL: Ignore our own logs to prevent infinite recursion
	if (Category == TEXT("LogCherryLink"))
	{
		return;
	}

	if (Verbosity > MinVerbosity)
	{
		return;
	}

	if (!ShouldLogCategory(Category))
	{
		return;
	}

	TSharedPtr<FCherryLinkServer> ServerPtr = Server.Pin();
	if (!ServerPtr.IsValid() || !ServerPtr->IsClientConnected())
	{
		return;
	}

	TSharedPtr<FJsonObject> Params = MakeShared<FJsonObject>();
	Params->SetStringField(TEXT("category"), Category.ToString());
	Params->SetStringField(TEXT("verbosity"), VerbosityToString(Verbosity));
	Params->SetStringField(TEXT("message"), Message);
	Params->SetStringField(TEXT("timestamp"), FDateTime::UtcNow().ToIso8601());
	Params->SetNumberField(TEXT("frame"), static_cast<double>(GFrameCounter));

	ServerPtr->SendNotification(TEXT("logging/message"), Params);
}

void FLogService::HandleRequest(const FString& JsonMessage)
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

	// Only handle logging/* methods
	if (!Method.StartsWith(TEXT("logging/")))
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

	if (Method == TEXT("logging/subscribe"))
	{
		UE_LOG(LogCherryLink, Log, TEXT("LogService: Client subscribed to logging"));
		bIsEnabled = true;

		TSharedPtr<FJsonObject> Result = MakeShared<FJsonObject>();
		Result->SetBoolField(TEXT("success"), true);
		ServerPtr->SendResponse(Id, MakeShared<FJsonValueObject>(Result));
	}
	else if (Method == TEXT("logging/unsubscribe"))
	{
		UE_LOG(LogCherryLink, Log, TEXT("LogService: Client unsubscribed from logging"));
		bIsEnabled = false;

		TSharedPtr<FJsonObject> Result = MakeShared<FJsonObject>();
		Result->SetBoolField(TEXT("success"), true);
		ServerPtr->SendResponse(Id, MakeShared<FJsonValueObject>(Result));
	}
}
