WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
TopQuestions AS (
    SELECT 
        PostId, Title, OwnerDisplayName, Score, ViewCount
    FROM 
        RankedPosts
    WHERE 
        Rank <= 10 AND PostId IN (SELECT PostId FROM Comments WHERE Score > 0)
),
TopAnswers AS (
    SELECT 
        p.Id AS AnswerId,
        p.Score AS AnswerScore,
        p.ViewCount AS AnswerViewCount,
        q.Title AS QuestionTitle,
        q.OwnerDisplayName AS QuestionOwnerDisplayName
    FROM 
        Posts p
    JOIN 
        Posts q ON p.ParentId = q.Id
    WHERE 
        p.PostTypeId = 2 AND 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
)
SELECT 
    tq.PostId,
    tq.Title AS QuestionTitle,
    tq.OwnerDisplayName AS QuestionOwner,
    ta.AnswerId,
    ta.AnswerScore,
    ta.AnswerViewCount,
    ta.QuestionTitle AS RelatedQuestionTitle,
    ta.QuestionOwnerDisplayName
FROM 
    TopQuestions tq
LEFT JOIN 
    TopAnswers ta ON tq.PostId = ta.AnswerId
ORDER BY 
    tq.Score DESC, tq.ViewCount DESC, tq.PostId;